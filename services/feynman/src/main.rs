mod config;
mod prompt_loader;

use crate::config::{Config, INPUT_CHUNK_SIZE, OUTPUT_CHUNK_SIZE, OUTPUT_LATENCY_MS};
use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FrameCount, StreamConfig};
use feynman_core::reviewer::{Reviewer, ReviewerClient};
use feynman_core::session_state::FeynmanSession;
use feynman_core::topic::{SubTopic, SubTopicList, Topic};
use feynman_native_utils::audio::REALTIME_API_PCM16_SAMPLE_RATE;
use openai_realtime::types::audio::Base64EncodedAudioBytes;
use openai_realtime::types::audio::{ServerVadTurnDetection, TurnDetection};
use ringbuf::traits::{Consumer, Producer, Split};
use rubato::Resampler;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::fmt::time::ChronoLocal;

pub enum Input {
    Audio(Vec<f32>),
    Initialize(),
    Initialized(),
    AISpeaking(),
    AISpeakingDone(),
    /// Command to the `client_handle` to create a spoken response from the AI.
    CreateSpokenResponse(String),
}

#[derive(Parser)]
struct Cli {
    /// The main topic to teach
    topic: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Load Configuration ---
    let config = Config::from_env().context("Failed to load application configuration")?;

    // --- 2. Initialize Logging ---
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_timer(ChronoLocal::rfc_3339())
        .init();

    tracing::info!("Configuration loaded successfully. Starting Feynman service...");

    // --- 3. Parse Command-Line Arguments ---
    let args = Cli::parse();

    // --- 4. Load Prompts ---
    let prompts = prompt_loader::load_prompts(Path::new("prompts"))
        .context("Failed to load LLM prompts")?;
    tracing::info!("Loaded {} prompts successfully.", prompts.len());

    // --- 5. Initialize API Clients ---
    let reviewer = Arc::new(ReviewerClient::new(
        config.openai_api_key.clone(),
        config.chat_model.clone(),
        prompts,
    ));

    // --- 6. Application Setup ---

    // This block sets up audio channels, gets an input device, configures it,
    // and prints the device information.
    // Audio channels for communication between tasks.
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Input>(1024);
    // Create the command channel to decouple core logic from the runtime.
    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel::<feynman_core::Command>(32);

    // Setup audio input device.
    let input =
        feynman_native_utils::device::get_or_default_input(None).context("Failed to get default audio input device")?;

    // Print out the supported configs for the input device.
    tracing::info!("Using input device: {:?}", &input.name()?);
    for config in input.supported_input_configs()? {
        tracing::debug!("Supported input config: {:?}", config);
    }

    // Get the default configuration for the audio input.
    let input_config = input
        .default_input_config()
        .context("Failed to get default input config")?;

    // Create an audio stream config using the default channels and sample rate, but with a fixed buffer size.
    let input_config = StreamConfig {
        channels: input_config.channels(),
        sample_rate: input_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(INPUT_CHUNK_SIZE as u32)),
    };
    // Get the number of input channels.
    let input_channel_count = input_config.channels as usize;
    tracing::info!("Input stream config: {:?}", &input_config);

    //----------------------------------------------------------------/
    // This block builds the input stream. An inline function processes raw audio data,
    // converts it to a mono f32 vector, and sends it over a channel.
    // The stream is then built and started.

    // Clone the audio input channel transmitter for the input callback.
    let audio_input = input_tx.clone();

    // This callback function processes audio data from the input stream.
    // It converts stereo to mono if necessary and sends the audio data over the channel.
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

    // Build the input stream.
    let input_stream = input.build_input_stream(
        &input_config,
        input_data_fn,
        move |err| tracing::error!("An error occurred on input stream: {}", err),
        None,
    )?;

    input_stream.play()?;
    let input_sample_rate = input_config.sample_rate.0 as f32;

    //------------------------------------------------------------/

    // Get the default output device.
    let output =
        feynman_native_utils::device::get_or_default_output(None).context("Failed to get default audio output device")?;

    tracing::info!("Using output device: {:?}", &output.name()?);
    for config in output.supported_output_configs()? {
        tracing::debug!("Supported output config: {:?}", config);
    }

    // Get the default output configuration.
    let output_config = output
        .default_output_config()
        .context("Failed to get default output config")?;
    // Create a stream config with a fixed buffer size.
    let output_config = StreamConfig {
        channels: output_config.channels(),
        sample_rate: output_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(OUTPUT_CHUNK_SIZE as u32)),
    };

    let output_channel_count = output_config.channels as usize;
    let output_sample_rate = output_config.sample_rate.0 as f32;
    tracing::info!("Output stream config: {:?}", &output_config);

    let audio_out_buffer =
        feynman_native_utils::audio::shared_buffer(output_sample_rate as usize * OUTPUT_LATENCY_MS);
    // Create a producer and consumer for the audio output buffer. This is used to receive audio from the AI and play it.
    let (mut audio_out_tx, mut audio_out_rx) = audio_out_buffer.split();

    let client_ctrl = input_tx.clone();
    // This callback function provides audio data to the output stream.
    // It pulls samples from the ring buffer and sends events to indicate if the AI is speaking.
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let mut sample_index = 0;
        let mut silence = 0;
        // Fill the output buffer with samples from the ring buffer.
        while sample_index < data.len() {
            // Get a single sample value.
            let sample = audio_out_rx.try_pop().unwrap_or(0.0);
            if sample == 0.0 {
                silence += 1;
            }
            // Left channel (ch:0).
            if sample_index < data.len() {
                data[sample_index] = sample;
                sample_index += 1;
            }
            // Right channel (ch:1), if it exists.
            if output_channel_count > 1 && sample_index < data.len() {
                data[sample_index] = sample;
                sample_index += 1;
            }
            // Ignore other channels.
            sample_index += output_channel_count.saturating_sub(2);
        }

        // At this point, the `data` buffer is filled.
        // Notify the client task when the AI is speaking or has finished.
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
    // Build the output stream.
    let output_stream = output.build_output_stream(
        &output_config,
        output_data_fn,
        move |err| tracing::error!("An error occurred on output stream: {}", err),
        None,
    )?;
    // Begin playing the output stream.
    output_stream.play()?;

    // OpenAI Realtime API
    // Connect to the API. The `realtime_api` client is used to send events.
    let mut realtime_api = openai_realtime::connect()
        .await
        .context("Failed to connect to OpenAI Realtime API")?;

    let topic = Topic {
        main_topic: args.topic,
    };

    tracing::info!("Generating subtopics for main topic: '{}'", topic.main_topic);
    let subtopic_names = reviewer.generate_subtopics(&topic.main_topic).await?;
    let subtopics: Vec<SubTopic> = subtopic_names.into_iter().map(SubTopic::new).collect();
    let subtopic_list = SubTopicList::new(subtopics);
    tracing::debug!("Generated subtopics: {:?}", subtopic_list.subtopics);

    // Create a resampler to configure the output sample rate.
    let mut out_resampler = feynman_native_utils::audio::create_resampler(
        REALTIME_API_PCM16_SAMPLE_RATE,
        output_sample_rate as f64,
        100,
    )?;

    // This channel receives base64 encoded audio from the server events task.
    let (post_tx, mut post_rx) = tokio::sync::mpsc::channel::<Base64EncodedAudioBytes>(100);

    let post_process = tokio::spawn(async move {
        // This task receives audio from the server, decodes, resamples, and pushes it to the output buffer.
        while let Some(audio) = post_rx.recv().await {
            // Decode audio into a vector of floats.
            let audio_bytes = feynman_native_utils::audio::decode(&audio);
            // Get the resampler's required chunk size.
            let chunk_size = out_resampler.input_frames_next();

            // Send the received audio to the audio buffer for playback.
            for samples in feynman_native_utils::audio::split_for_chunks(&audio_bytes, chunk_size) {
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
    // Create a subscriber for server events.
    let mut server_events = realtime_api
        .server_events()
        .await
        .context("Failed to get server events channel")?;
    let reviewer2 = reviewer.clone();
    let command_tx_for_server = command_tx.clone();

    let server_handle = tokio::spawn(async move {
        let mut session = FeynmanSession::new(subtopic_list);

        // Receive and process events from the server.
        while let Ok(e) = server_events.recv().await {
            // Match on the event type.
            match e {
                // When the session is created, send an `Initialize` event to the client task.
                openai_realtime::types::events::ServerEvent::SessionCreated(data) => {
                    tracing::info!("Session created: {:?}", data.session());
                    if let Err(e) = client_ctrl2.try_send(Input::Initialize()) {
                        tracing::warn!("Failed to send initialized event to client: {:?}", e);
                    }
                }
                // When the session is updated, send an `Initialized` event.
                openai_realtime::types::events::ServerEvent::SessionUpdated(data) => {
                    tracing::info!("Session updated: {:?}", data.session());
                    if let Err(e) = client_ctrl2.try_send(Input::Initialized()) {
                        tracing::warn!("Failed to send initialized event to client: {:?}", e);
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
                    FeynmanSession::process_segment(&mut session, &*reviewer2, segment, command_tx_for_server.clone()).await;
                }
                
                // If we receive response audio, send it to the post-processing channel.
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

    // Create a resampler to transform audio from the input device to the sample rate OpenAI's API requires.
    let mut in_resampler = feynman_native_utils::audio::create_resampler(
        input_sample_rate as f64,
        REALTIME_API_PCM16_SAMPLE_RATE,
        INPUT_CHUNK_SIZE,
    )?;

    // This task handles commands from the core logic, executing side effects.
    let input_tx_for_cmd_handler = input_tx.clone();
    let command_handler = tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            match command {
                feynman_core::Command::SpeakText(text) => {
                    tracing::info!("COMMAND RECEIVED: Speak Text: '{}'", text);
                    // Send a command to the client_handle task, telling it to
                    // create a conversation item and trigger TTS.
                    if let Err(e) = input_tx_for_cmd_handler
                        .send(Input::CreateSpokenResponse(text))
                        .await
                    {
                        tracing::error!("Failed to send CreateSpokenResponse command: {:?}", e);
                    }
                }
                feynman_core::Command::SessionComplete(message) => {
                    tracing::info!("COMMAND RECEIVED: Session Complete: '{}'", message);
                    // Here you could break the loop or trigger a shutdown.
                }
            }
        }
    });

    // This task handles client-side logic: sending user audio and managing state.
    let client_handle = tokio::spawn(async move {
        let mut ai_speaking = false;
        let mut initialized = false;
        let mut buffer: VecDeque<f32> = VecDeque::with_capacity(INPUT_CHUNK_SIZE * 2);

        // This inner function allows us to use the `?` operator inside the loop.
        // The loop itself will continue, but a single failed operation will be logged
        // and won't crash the task.
        async fn handle_input(
            i: Input,
            realtime_api: &mut openai_realtime::Client,
            initialized: &mut bool,
            ai_speaking: &mut bool,
            buffer: &mut VecDeque<f32>,
            in_resampler: &mut impl Resampler<f32>,
        ) -> Result<()> {
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
                    // Once a connection has been established, update the session with custom parameters.
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
                        .context("Failed to initialize session")?;
                }
                Input::Initialized() => {
                    tracing::info!("Session initialized successfully.");
                    *initialized = true;
                }
                Input::AISpeaking() => {
                    if !*ai_speaking {
                        tracing::debug!("AI speaking...");
                    }
                    buffer.clear();
                    *ai_speaking = true;
                }
                Input::AISpeakingDone() => {
                    if *ai_speaking {
                        tracing::debug!("AI speaking done");
                    }
                    *ai_speaking = false;
                }
                Input::Audio(audio) => {
                    if *initialized && !*ai_speaking {
                        buffer.extend(audio);
                        let mut resampled: Vec<f32> = vec![];
                        while buffer.len() >= INPUT_CHUNK_SIZE {
                            let audio_chunk: Vec<f32> = buffer.drain(..INPUT_CHUNK_SIZE).collect();
                            if let Ok(resamples) = in_resampler.process(&[audio_chunk.as_slice()], None)
                            {
                                if let Some(resamples) = resamples.first() {
                                    resampled.extend(resamples.iter().cloned());
                                }
                            }
                        }
                        if !resampled.is_empty() {
                            let audio_bytes = feynman_native_utils::audio::encode(&resampled);
                            let audio_bytes = Base64EncodedAudioBytes::from(audio_bytes);
                            realtime_api
                                .append_input_audio_buffer(audio_bytes.clone())
                                .await
                                .context("Failed to send audio buffer")?;
                        }
                    }
                }
                Input::CreateSpokenResponse(text) => {
                    // 1. Create a message item for the system/assistant to say.
                    //    We use the "system" role to inject instructions for the AI to speak.
                    let item = openai_realtime::types::MessageItem::builder()
                        .with_role(openai_realtime::types::MessageRole::System)
                        .with_input_text(&text)
                        .build();

                    // 2. Send this item to the conversation history.
                    realtime_api
                        .create_conversation_item(openai_realtime::types::Item::Message(item))
                        .await
                        .context("Failed to create conversation item for AI speech")?;

                    // 3. Trigger a response, which will cause the OpenAI server
                    //    to read the last message (the one we just sent) and generate audio for it.
                    realtime_api
                        .create_response()
                        .await
                        .context("Failed to trigger response for AI speech")?;
                }
            }
            Ok(())
        }

        // Receive and process inputs from the audio callbacks and server event handler.
        while let Some(i) = input_rx.recv().await {
            if let Err(e) = handle_input(
                i,
                &mut realtime_api,
                &mut initialized,
                &mut ai_speaking,
                &mut buffer,
                &mut in_resampler,
            )
            .await
            {
                // Log the error from the handler, but don't crash the whole task.
                tracing::error!("Error in client handler: {:?}", e);
            }
        }
    });

    tokio::select! {
        _ = post_process => {},
        _ = server_handle => {},
        _ = client_handle => {},
        _ = command_handler => {},
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl-C, shutting down...");
        }
    }
    tracing::info!("Shutting down...");
    Ok(())
}