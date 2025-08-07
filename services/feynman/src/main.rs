mod config;

use crate::config::{Config, INPUT_CHUNK_SIZE, OUTPUT_CHUNK_SIZE, OUTPUT_LATENCY_MS};
use anyhow::Result;
use clap::Parser;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FrameCount, StreamConfig};
use feynman_core::reviewer::ReviewerClient;
use feynman_core::session_state::FeynmanSession;
use feynman_core::topic::{SubTopic, SubTopicList, Topic};
use openai_realtime::types::audio::Base64EncodedAudioBytes;
use openai_realtime::types::audio::{ServerVadTurnDetection, TurnDetection};
use openai_realtime::utils;
use openai_realtime::utils::audio::REALTIME_API_PCM16_SAMPLE_RATE;
use ringbuf::traits::{Consumer, Producer, Split};
use rubato::Resampler;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tracing_subscriber::fmt::time::ChronoLocal;

pub enum Input {
    Audio(Vec<f32>),
    Initialize(),
    Initialized(),
    AISpeaking(),
    AISpeakingDone(),
    CreateConversationItem(openai_realtime::types::Item),
}

#[derive(Parser)]
struct Cli {
    /// The main topic to teach
    topic: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Load Configuration ---
    let config = Config::from_env()?;

    // --- 2. Initialize Logging ---
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_timer(ChronoLocal::rfc_3339())
        .init();

    tracing::info!("Configuration loaded successfully. Starting Feynman service...");

    // --- 3. Parse Command-Line Arguments ---
    let args = Cli::parse();

    // --- 4. Initialize API Clients ---
    let reviewer = Arc::new(ReviewerClient::new(
        config.openai_api_key.clone(),
        config.chat_model.clone(),
    ));

    // --- 5. Application Setup ---

    //the code in this block does the following: sets up our audio channels, gets an input device,
    //sets the configs for the input device, and then prints out the device with its configs
    //audio channels
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Input>(1024);

    // Setup audio input device
    let input = utils::device::get_or_default_input(None).expect("failed to get input device");

    //print out the supported configs for input
    tracing::info!("Using input device: {:?}", &input.name()?);
    for config in input.supported_input_configs()? {
        tracing::debug!("Supported input config: {:?}", config);
    }

    //get the default configs for the audio
    let input_config = input
        .default_input_config()
        .expect("failed to get default input config");

    //create a audio stream config using channels and sample rate default,
    let input_config = StreamConfig {
        channels: input_config.channels(),
        sample_rate: input_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(INPUT_CHUNK_SIZE as u32)),
    };
    //get the number of input channels
    let input_channel_count = input_config.channels as usize;

    tracing::info!("Input stream config: {:?}", &input_config);

    //----------------------------------------------------------------/
    //here we build out our input stream by using an inline function to transform audio into a float
    //vector and then send this audio using our clone of the input transmitter
    //we then build the input stream using the input device config, inline funciton, and an error statement
    //we then play this input stream to start listening to audio

    //create a clone of the audio input channel we will be using
    let audio_input = input_tx.clone();

    //inline function to convert data from stereo to mono or take mono and convert it to vector of floats
    //we then take this buffer and send it through input audio channel
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let audio = if input_channel_count > 1 {
            data.chunks(input_channel_count)
                .map(|c| c.iter().sum::<f32>() / input_channel_count as f32)
                .collect::<Vec<f32>>()
        } else {
            data.to_vec()
        };
        if let Err(e) = audio_input.try_send(Input::Audio(audio)) {
            tracing::warn!("Failed to send audio data to buffer: {:?}", e);
        }
    };

    //build the input stream
    let input_stream = input.build_input_stream(
        &input_config,
        input_data_fn,
        move |err| tracing::error!("An error occurred on input stream: {}", err),
        None,
    )?;

    input_stream.play()?;
    let input_sample_rate = input_config.sample_rate.0 as f32;

    //------------------------------------------------------------/

    //get the default output device
    let output = utils::device::get_or_default_output(None).expect("failed to get output device");

    tracing::info!("Using output device: {:?}", &output.name()?);
    for config in output.supported_output_configs()? {
        tracing::debug!("Supported output config: {:?}", config);
    }

    //set the output device configs to the default output config
    let output_config = output
        .default_output_config()
        .expect("failed to get default output config");
    //set the buffersize, channels and sample rate
    let output_config = StreamConfig {
        channels: output_config.channels(),
        sample_rate: output_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(OUTPUT_CHUNK_SIZE as u32)),
    };

    let output_channel_count = output_config.channels as usize;
    let output_sample_rate = output_config.sample_rate.0 as f32;
    tracing::info!("Output stream config: {:?}", &output_config);

    let audio_out_buffer =
        utils::audio::shared_buffer(output_sample_rate as usize * OUTPUT_LATENCY_MS);
    //create a producer and consumer for audio which will be used to receive audio and then play it
    let (mut audio_out_tx, mut audio_out_rx) = audio_out_buffer.split();

    let client_ctrl = input_tx.clone();
    //inline function to get data from the audio buffer and then indicates when the ai is speaking or done speaking
    //cpal library plays the audio data we pull from the audio ring buffer, and we use the input_tx clone
    //to send whether or not ai is speaking.
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let mut sample_index = 0;
        let mut silence = 0;
        //while data is not full
        while sample_index < data.len() {
            //get single sample value
            let sample = audio_out_rx.try_pop().unwrap_or(0.0);

            if sample == 0.0 {
                silence += 1;
            }

            // L channel (ch:0)
            if sample_index < data.len() {
                data[sample_index] = sample;
                sample_index += 1;
            }
            // R channel (ch:1)
            if output_channel_count > 1 && sample_index < data.len() {
                data[sample_index] = sample;
                sample_index += 1;
            }

            // ignore other channels
            sample_index += output_channel_count.saturating_sub(2);
        }

        //notify us when ai is speaking or done speaking
        let client_ctrl = client_ctrl.clone();
        if silence == (data.len() / output_channel_count) {
            if let Err(e) = client_ctrl.try_send(Input::AISpeakingDone()) {
                tracing::warn!("Failed to send speaking done event to client: {:?}", e);
            }
        } else {
            if let Err(e) = client_ctrl.try_send(Input::AISpeaking()) {
                tracing::warn!("Failed to send speaking event to client: {:?}", e);
            }
        }
    };
    //build the output stream
    let output_stream = output.build_output_stream(
        &output_config,
        output_data_fn,
        move |err| tracing::error!("An error occurred on output stream: {}", err),
        None,
    )?;

    //begin playing the output stream
    output_stream.play()?;

    // OpenAI Realtime API
    //connect with default configs now realtime_api is a client that we can use to send events
    let mut realtime_api = openai_realtime::connect()
        .await
        .expect("failed to connect to OpenAI Realtime API");

    let topic = Topic {
        main_topic: args.topic,
    };

    tracing::info!("Generating subtopics for main topic: '{}'", topic.main_topic);
    let subtopic_names = match reviewer.generate_subtopics(&topic.main_topic).await {
        Ok(names) => names,
        Err(e) => {
            tracing::error!("Fatal: Could not generate subtopics: {:?}", e);
            return Ok(());
        }
    };
    let subtopics: Vec<SubTopic> = subtopic_names.into_iter().map(SubTopic::new).collect();
    let subtopic_list = SubTopicList::new(subtopics);
    tracing::debug!("Generated subtopics: {:?}", subtopic_list.subtopics);

    //set the resampler to configure theoutput sample rate
    let mut out_resampler = utils::audio::create_resampler(
        REALTIME_API_PCM16_SAMPLE_RATE,
        output_sample_rate as f64,
        100,
    )?;

    //base 64 channels that I am going to assume this is used to recieve audio from openai
    let (post_tx, mut post_rx) = tokio::sync::mpsc::channel::<Base64EncodedAudioBytes>(100);

    //
    let post_process = tokio::spawn(async move {
        //recieve audio through post channel
        while let Some(audio) = post_rx.recv().await {
            //decode audio into vector of floats
            let audio_bytes = utils::audio::decode(&audio);
            //get chunk size
            let chunk_size = out_resampler.input_frames_next();

            //here we are sending audio that we recieve through the post_rx channel to the audio buff which will be processed using output channel
            for samples in utils::audio::split_for_chunks(&audio_bytes, chunk_size) {
                if let Ok(resamples) = out_resampler.process(&[samples.as_slice()], None) {
                    if let Some(resamples) = resamples.first() {
                        for resample in resamples {
                            if let Err(e) = audio_out_tx.try_push(*resample) {
                                tracing::warn!("Failed to push samples to buffer: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    });

    let client_ctrl2 = input_tx.clone();
    //create a server events subscriber, subscribing to the server transmitter
    let mut server_events = realtime_api
        .server_events()
        .await
        .expect("failed to get server events");
    let reviewer2 = reviewer.clone();

    let server_handle = tokio::spawn(async move {
        let mut pending_interrupts: HashSet<String> = Default::default();
        // Add this flag for awaiting item creation
        let mut awaiting_item_creation = false;

        let mut session = FeynmanSession::new(subtopic_list);

        //recieve events
        while let Ok(e) = server_events.recv().await {
            // tracing::trace!("Server Event: {:?}", &e);
            //match the event
            match e {
                //send session created to the input channel, here we call intiliaze
                openai_realtime::types::events::ServerEvent::SessionCreated(data) => {
                    tracing::info!("Session created: {:?}", data.session());
                    if let Err(e) = client_ctrl2.try_send(Input::Initialize()) {
                        tracing::warn!("Failed to send initialized event to client: {:?}", e);
                    }
                }
                //session updated here we once again call intiliaze
                openai_realtime::types::events::ServerEvent::SessionUpdated(data) => {
                    tracing::info!("Session updated: {:?}", data.session());
                    if let Err(e) = client_ctrl2.try_send(Input::Initialized()) {
                        tracing::warn!("Failed to send initialized event to client: {:?}", e);
                    }
                }
                openai_realtime::types::events::ServerEvent::ConversationItemCreated(data) => {
                    if awaiting_item_creation {
                        let item_id = data.item().id().to_string();
                        pending_interrupts.insert(item_id);
                        awaiting_item_creation = false; // <-- Reset the flag
                    }
                }
                openai_realtime::types::events::ServerEvent::InputAudioBufferSpeechStarted(
                    data,
                ) => {
                    tracing::debug!("User speech started: {:?}", data);
                }
                openai_realtime::types::events::ServerEvent::InputAudioBufferSpeechStopped(
                    data,
                ) => {
                    tracing::debug!("User speech stopped: {:?}", data);
                }
                openai_realtime::types::events::ServerEvent::ConversationItemInputAudioTranscriptionCompleted(data ) => {
                    let segment = data.transcript().trim().to_owned();
                    tracing::info!("User said: \"{}\"", segment);
                    FeynmanSession::process_segment(&mut session, &reviewer2, segment).await;
                }
                
                //if we get response audio send it to the post channel
                openai_realtime::types::events::ServerEvent::ResponseAudioDelta(data) => {
           
                    if let Err(e) = post_tx.send(data.delta().to_string()).await {
                        tracing::warn!("Failed to send audio data to resampler: {:?}", e);
                    }
                }
                openai_realtime::types::events::ServerEvent::ResponseCreated(data) => {
                    tracing::debug!("Response created: {:?}", data.response());
                }
                openai_realtime::types::events::ServerEvent::ResponseAudioTranscriptDone(data) => {
                    tracing::info!("AI said: {:?}", data.transcript());
                }
                openai_realtime::types::events::ServerEvent::ResponseDone(data) => {
                    tracing::debug!("Response done. Usage: {:?}", data.response().usage());
                }
                openai_realtime::types::events::ServerEvent::Close { reason } => {
                    tracing::info!("Connection closed: {:?}", reason);
                    break;
                }
                _ => {}
            }
        }
    });

    //create a resampler to transform audio from the input audio to whatever rate openai needs
    let mut in_resampler = utils::audio::create_resampler(
        input_sample_rate as f64,
        REALTIME_API_PCM16_SAMPLE_RATE,
        INPUT_CHUNK_SIZE,
    )?;

    // client_events for audio
    //taking the audio from user microphone and sending it
    let client_handle = tokio::spawn(async move {
        let mut ai_speaking = false;
        let mut initialized = false;
        let mut buffer: VecDeque<f32> = VecDeque::with_capacity(INPUT_CHUNK_SIZE * 2);

        //grab audio that was sent to input channel
        while let Some(i) = input_rx.recv().await {
            match i {
                Input::Initialize() => {
                    let instructions = r#"You are a curious student in a Feynman session.
                        - You know ONLY what the teacher just said.
                        - When you receive one or more questions (one per line), read them and ask them out loud, one-by-one, in order.
                        - Do NOT answer questions or add information. Do NOT speculate or rephrase the teacher's content.
                        - If a single clarification question is received, just ask that one and stop.
                        - Keep each spoken question concise and natural."#;

                    let turn_detection = TurnDetection::ServerVad(
                        ServerVadTurnDetection::default()
                            .with_interrupt_response(true)
                            .with_create_response(false),
                    );
                    //once a connection has be established update the session with the custom parameters
                    tracing::info!("Initializing session with OpenAI...");
                    let session = openai_realtime::types::Session::new()
                        .with_modalities_enable_audio()
                        .with_instructions(instructions)
                        .with_voice(openai_realtime::types::audio::Voice::Alloy)
                        .with_input_audio_transcription_enable(
                            openai_realtime::types::audio::TranscriptionModel::Whisper,
                        )
                        .with_turn_detection_enable(turn_detection)
                        .build();
                    tracing::debug!("Session config: {:?}", serde_json::to_string(&session)?);
                    realtime_api
                        .update_session(session)
                        .await
                        .expect("failed to init session");
                }
                Input::Initialized() => {
                    tracing::info!("Session initialized successfully.");
                    initialized = true;
                }
                Input::AISpeaking() => {
                    if !ai_speaking {
                        tracing::debug!("AI speaking...");
                    }
                    buffer.clear();
                    ai_speaking = true;
                }
                Input::AISpeakingDone() => {
                    if ai_speaking {
                        tracing::debug!("AI speaking done");
                    }
                    ai_speaking = false;
                }
                Input::Audio(audio) => {
                    if initialized && !ai_speaking {
                        for sample in audio {
                            buffer.push_back(sample);
                        }
                        let mut resampled: Vec<f32> = vec![];
                        while buffer.len() >= INPUT_CHUNK_SIZE {
                            let audio: Vec<f32> = buffer.drain(..INPUT_CHUNK_SIZE).collect();
                            if let Ok(resamples) = in_resampler.process(&[audio.as_slice()], None) {
                                if let Some(resamples) = resamples.first() {
                                    resampled.extend(resamples.iter().cloned());
                                }
                            }
                        }
                        if resampled.is_empty() {
                            continue;
                        }
                        let audio_bytes = utils::audio::encode(&resampled);
                        let audio_bytes = Base64EncodedAudioBytes::from(audio_bytes);
                        realtime_api
                            .append_input_audio_buffer(audio_bytes.clone())
                            .await
                            .expect("failed to send audio");
                    }
                }

                Input::CreateConversationItem(item) => {
                    if let Err(e) = realtime_api.create_conversation_item(item).await {
                        tracing::error!("Error creating conversation item: {:?}", e);
                    }
                    if let Err(e) = realtime_api.create_response().await {
                        tracing::error!("Error creating response for conversation item");
                    }
                }
            }
        }
        // Add a return type for the async block to satisfy the compiler
        Ok::<(), anyhow::Error>(())
    });

    tokio::select! {
        _ = post_process => {},
        _ = server_handle => {},
        _ = client_handle => {},
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl-C, shutting down...");
        }
    }
    tracing::info!("Shutting down...");
    Ok(())
}