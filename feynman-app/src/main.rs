use std::collections::{VecDeque, HashSet};
use serde::Deserialize;
use cpal::{FrameCount, StreamConfig};
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::{
    traits::{Consumer, Producer, Split},
};

use rubato::Resampler;
use tracing::Level;
use tracing_subscriber::fmt::time::ChronoLocal;
use openai_realtime::types::audio::Base64EncodedAudioBytes;
use openai_realtime::utils::audio::REALTIME_API_PCM16_SAMPLE_RATE;
use openai_realtime::utils as utils;
use openai_realtime::types::audio::{TurnDetection, ServerVadTurnDetection};

mod topic;
mod reviewer;


use topic::{TopicBuffer, TopicChange};
use reviewer::ReviewerClient;


const INPUT_CHUNK_SIZE: usize = 1024;
const OUTPUT_CHUNK_SIZE: usize = 1024;
const OUTPUT_LATENCY_MS: usize =   1000;

pub enum Input {
    Audio(Vec<f32>),
    Initialize(),
    Initialized(),
    AISpeaking(),
    AISpeakingDone(),
    CreateConversationItem(openai_realtime::types::Item),
}


#[tokio::main]
async fn main() {
    //load the environment variables
    dotenvy::dotenv_override().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("key not set");
    let model = "gpt-4o".to_string();
    let reviewer = ReviewerClient::new(api_key, model);
    let reviewer = std::sync::Arc::new(reviewer);


    
    //create tracing subcsriber to tracking debug statements with timestamps
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_timer(ChronoLocal::rfc_3339())
        .init();
    //-------------------------------------------------------------------------------/
    //the code in this block does the following: sets up our audio channels, gets an input device,
    //sets the configs for the input device, and then prints out the device with its configs
    //audio channels
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Input>(1024);

    // Setup audio input device
    let input = utils::device::get_or_default_input(None).expect("failed to get input device");

    //print out the supported configs for input
    println!("input: {:?}", &input.name().unwrap());
    input.supported_input_configs().expect("failed to get supported input configs")
        .for_each(|c| println!("supported input config: {:?}", c));

    //get the default configs for the audio
    let input_config = input.default_input_config().expect("failed to get default input config");
    
    //create a audio stream config using channels and sample rate default, 
    let input_config = StreamConfig {
        channels: input_config.channels(),
        sample_rate: input_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(INPUT_CHUNK_SIZE as u32)),
    };
    //get the number of input channels
    let input_channel_count = input_config.channels as usize;

    //print the input device and configs
    println!("input: device={:?}, config={:?}", &input.name().unwrap(), &input_config);

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
            data.chunks(input_channel_count).map(|c| {
                c.iter().sum::<f32>() / input_channel_count as f32
            }).collect::<Vec<f32>>()
        } else {
            data.to_vec()
        };
        if let Err(e) = audio_input.try_send(Input::Audio(audio)) {
            eprintln!("Failed to send audio data to buffer: {:?}", e);
        }
    };

    //build the input stream
    let input_stream = input.build_input_stream(
        &input_config,
        input_data_fn,
        move |err| eprintln!("an error occurred on input stream: {}", err),
        None,
    ).expect("failed to build input stream");

    input_stream.play().expect("failed to play input stream");
    let _input_channel_count = input_config.channels as usize;
    let input_sample_rate = input_config.sample_rate.0 as f32;

    //------------------------------------------------------------/

    //get the default output device
    let output = utils::device::get_or_default_output(None).expect("failed to get output device");

    //get the name of the output device
    println!("output: {:?}", &output.name().unwrap());

    //output the output devices support configs
    output.supported_output_configs().expect("failed to get supported output configs")
        .for_each(|c| println!("supported output config: {:?}", c));

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
    println!("output: device={:?}, config={:?}", &output.name().unwrap(), &output_config);


    let audio_out_buffer = utils::audio::shared_buffer(output_sample_rate as usize * OUTPUT_LATENCY_MS);
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
        //at this point we have a filled data array

        // println!("silence: {:?}", silence);
        //notify us when ai is speaking or done speaking
        let client_ctrl = client_ctrl.clone();
        if silence == (data.len() / output_channel_count) {
            if let Err(e) = client_ctrl.try_send(Input::AISpeakingDone()) {
                eprintln!("Failed to send speaking done event to client: {:?}", e);
            }
        } else {
            // println!("speaking..., silence: {:?}, len: {}", silence, data.len());
            if let Err(e) = client_ctrl.try_send(Input::AISpeaking()) {
                eprintln!("Failed to send speaking event to client: {:?}", e);
            }
        }
    };
    //build the output stream
    let output_stream = output.build_output_stream(
        &output_config,
        output_data_fn,
        move |err| eprintln!("an error occurred on output stream: {}", err),
        None,
    ).expect("failed to build output stream");

    //begin playing the output stream
    output_stream.play().expect("failed to play output stream");


    // OpenAI Realtime API
    //connect with default configs now realtime_api is a client that we can use to send events
    let mut realtime_api = openai_realtime::connect().await.expect("failed to connect to OpenAI Realtime API");

    //set the resampler to configure theoutput sample rate
    let mut out_resampler = utils::audio::create_resampler(
        REALTIME_API_PCM16_SAMPLE_RATE,
        output_sample_rate as f64,
        100,
    ).expect("failed to create resampler for output");

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
                                eprintln!("Failed to push samples to buffer: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    });

    let client_ctrl2 = input_tx.clone();
    //create a server events subscriber, subscribing to the server transmitter
    let mut server_events = realtime_api.server_events().await.expect("failed to get server events");
    let reviewer2 = reviewer.clone();

    let server_handle = tokio::spawn(async move {
        //hold the current topic and its segments with topic buffer
        let mut current_topic = TopicBuffer {topic: String::new(), segments: Vec::new()};
        //hold past topics in a vector of topic buffers
        let mut past_topics: Vec<TopicBuffer> = Vec::new();
        //hold item id's of created interrupts as to only play those ones
        let mut pending_interrupts: HashSet<String> = Default::default();
        // Add this flag for awaiting item creation
        let mut awaiting_item_creation = false;

        //recieve events
        while let Ok(e) = server_events.recv().await {
            // println!("server_events: {:?}", &e);
            //match the event
            match e {
                //send session created to the input channel, here we call intiliaze
                openai_realtime::types::events::ServerEvent::SessionCreated(data) => {
                    println!("session created: {:?}", data.session());
                    if let Err(e) = client_ctrl2.try_send(Input::Initialize()) {
                        eprintln!("Failed to send initialized event to client: {:?}", e);
                    }
                }
                //session updated here we once again call intiliaze
                openai_realtime::types::events::ServerEvent::SessionUpdated(data) => {
                    println!("session updated: {:?}", data.session());
                    if let Err(e) = client_ctrl2.try_send(Input::Initialized()) {
                        eprintln!("Failed to send initialized event to client: {:?}", e);
                    }
                }
                openai_realtime::types::events::ServerEvent::ConversationItemCreated(data) => {
                    if awaiting_item_creation {
                        let item_id = data.item().id().to_string();
                        pending_interrupts.insert(item_id);
                        awaiting_item_creation = false; // <-- Reset the flag
                    }
                }
                openai_realtime::types::events::ServerEvent::InputAudioBufferSpeechStarted(data) => {
                    println!("speech started: {:?}", data);
                }
                openai_realtime::types::events::ServerEvent::InputAudioBufferSpeechStopped(data) => {
                    println!("speech stopped: {:?}", data);
                }
                openai_realtime::types::events::ServerEvent::ConversationItemInputAudioTranscriptionCompleted(data ) => {
                    let segment = data.transcript().trim().to_owned();

                    current_topic.segments.push(segment.clone());

                    let (topic_change, new_topic) = {
                        let context = current_topic.segments.join(" ");
                        
                        let response = match reviewer2
                            .looks_like_topic_change(&context, &segment)
                            .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("Error in looks_like_topic_change: {:?}", e);
                                continue;
                            }
                        };

                        let response_json = response
                            .trim()
                            .trim_start_matches("```json")
                            .trim_start_matches("```")
                            .trim_end_matches("```")
                            .trim();

                        // Print for debug
                        println!("TopicChange response (stripped): {:?}", response_json);
                        let result: TopicChange = match serde_json::from_str(&response_json) {
                            Ok(r) => r,
                            Err(e) => {
                                println!("TopicChange response: {:?}", response_json);
                                eprintln!("Error parsing TopicChange: {:?}", e);
                                continue;
                            }
                        };
                        (result.topic_change, result.new_topic)
                    };

                    if topic_change {
                        // Analyze the buffered topic with ReviewerClient
                        let analysis = match reviewer2
                            .analyze_topic(&current_topic.segments.join(" "), &current_topic.topic)
                            .await
                        {
                            Ok(a) => a,
                            Err(e) => {
                                eprintln!("Error in analyze_topic: {:?}", e);
                                continue;
                            }
                        };

                        if analysis != "OK" {
                            // This is where you send the AI question interrupt
                            let message = openai_realtime::types::MessageItem::builder()
                                .with_role(openai_realtime::types::MessageRole::Assistant)
                                .with_input_text(&analysis)
                                .build();
                            if let Err(e) = client_ctrl2.try_send(Input::CreateConversationItem(openai_realtime::types::Item::Message(message))) {
                                eprintln!("Failed to send CreateConversationItem to client: {:?}", e);
                            }
                            // Set the flag to wait for ConversationItemCreated event
                            awaiting_item_creation = true;
                        }

                        // Save current topic buffer and start a new one
                        past_topics.push(current_topic);
                        current_topic = TopicBuffer {
                            topic: new_topic.unwrap_or_default(),
                            segments: vec![segment],
                        };
                    }
                }
                //if we get response audio send it to the post channel
                openai_realtime::types::events::ServerEvent::ResponseAudioDelta(data) => {
                    let item_id = data.item_id();
                    if pending_interrupts.contains(item_id){
                        if let Err(e) = post_tx.send(data.delta().to_string()).await {
                            eprintln!("Failed to send audio data to resampler: {:?}", e);
                        }
                    }
                }
                // openai_realtime::types::events::ServerEvent::ResponseTextDone(data) => {
                //     println!("text: {:?}", data.text());
                // }
                openai_realtime::types::events::ServerEvent::ResponseCreated(data ) => {
                    println!("response created: {:?}", data.response());
                }
                openai_realtime::types::events::ServerEvent::ResponseAudioTranscriptDone(data) => {
                    println!("AI: {:?}", data.transcript());
                }
                // openai_realtime::types::events::ServerEvent::ResponseAudioDone(data ) => {
                //     println!("audio done: {:?}", data);
                // }
                openai_realtime::types::events::ServerEvent::ResponseDone(data) => {
                    println!("usage: {:?}", data.response().usage());
                    println!("output: {:?}", data.response().outputs());
                }
                openai_realtime::types::events::ServerEvent::Close { reason } => {
                    println!("close: {:?}", reason);
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
    ).expect("failed to create resampler for input");

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

                    let turn_detection = TurnDetection::ServerVad(ServerVadTurnDetection::default().with_interrupt_response(true).with_create_response(false));
                    //once a connection has be established update the session with the custom parameters
                    println!("initializing...");
                    let session = openai_realtime::types::Session::new()
                        .with_modalities_enable_audio()
                        .with_voice(openai_realtime::types::audio::Voice::Alloy)
                        .with_input_audio_transcription_enable(openai_realtime::types::audio::TranscriptionModel::Whisper)
                        .with_turn_detection_enable(turn_detection)
                        .build();
                    println!("session config: {:?}", serde_json::to_string(&session).unwrap());
                    realtime_api.update_session(session).await.expect("failed to init session");
                }
                Input::Initialized() => {
                    println!("initialized");
                    // let config = openai_realtime::types::Session::new()
                    //     .with_modalities_enable_audio()
                    //     .with_instructions("Please greeting in Japanese")
                    //     .build();
                    // realtime_api.create_response_with_config(config).await.expect("failed to send message");
                    initialized = true;
                }
                Input::AISpeaking() => {
                    if !ai_speaking {
                        println!("AI speaking...");
                    }
                    buffer.clear();
                    ai_speaking = true;
                }
                Input::AISpeakingDone() => {
                    if ai_speaking {
                        println!("AI speaking done");
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
                        realtime_api.append_input_audio_buffer(audio_bytes.clone()).await.expect("failed to send audio");

                    }
                }

                Input::CreateConversationItem(item) => {
                if let Err(e) = realtime_api.create_conversation_item(item).await {
                    eprintln!("Error creating conversation item: {:?}", e);
                }
                if let Err(e) = realtime_api.create_response().await{
                    eprintln!("Error creating response for conversation item");
                }
                }
            }
        }
     });

    tokio::select! {
        _ = post_process => {},
        _ = server_handle => {},
        _ = client_handle => {},
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl-C, shutting down...");
        }
    }
    println!("Shutting down...");
}