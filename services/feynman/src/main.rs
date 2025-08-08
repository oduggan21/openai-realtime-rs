mod config;
mod gemini_adapter;
mod openai_adapter;
mod prompt_loader;

use crate::config::{
    Config, INPUT_CHUNK_SIZE, OUTPUT_CHUNK_SIZE, OUTPUT_LATENCY_MS, RealtimeProvider,
};
use crate::openai_adapter::OpenAIAdapter;
use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FrameCount, StreamConfig};
use feynman_core::gemini_reviewer::GeminiReviewer;
use feynman_core::generic_types::{GenericServerEvent, GenericSessionConfig};
use feynman_core::realtime_api::RealtimeApi;
use feynman_core::reviewer::{OpenAIReviewer, Reviewer};
use feynman_core::session_state::FeynmanSession;
use feynman_core::topic::{SubTopic, SubTopicList, Topic};
use feynman_native_utils::audio::REALTIME_API_PCM16_SAMPLE_RATE;
use gemini_adapter::GeminiAdapter;
use openai_realtime::types::audio::Base64EncodedAudioBytes;
use ringbuf::traits::{Consumer, Producer, Split};
use rubato::Resampler;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::fmt::time::ChronoLocal;

pub enum Input {
    Audio(Vec<f32>),
    AISpeaking(),
    AISpeakingDone(),
    /// Command to the `client_handle` to create a spoken response from the AI.
    /// This triggers a TTS synthesis and playback flow.
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
    let prompts =
        prompt_loader::load_prompts(Path::new("prompts")).context("Failed to load LLM prompts")?;
    tracing::info!("Loaded {} prompts successfully.", prompts.len());

    // --- 5. Initialize API Clients ---
    let (mut realtime_api, reviewer): (Box<dyn RealtimeApi>, Arc<dyn Reviewer>) =
        match config.provider {
            RealtimeProvider::OpenAI => {
                tracing::info!("Using OpenAI Provider for Realtime and Reviewer");
                let api_key = config
                    .openai_api_key
                    .context("OPENAI_API_KEY must be set for openai provider")?;

                let adapter = OpenAIAdapter::new(api_key.clone()).await?;
                let reviewer = OpenAIReviewer::new(api_key, config.chat_model.clone(), prompts);

                (Box::new(adapter), Arc::new(reviewer))
            }
            RealtimeProvider::Gemini => {
                tracing::info!("Using Gemini Provider for Realtime and Reviewer");
                let api_key = config
                    .gemini_api_key
                    .context("GEMINI_API_KEY must be set for gemini provider")?;

                let adapter = GeminiAdapter::new(&api_key).await?;
                // Using the simulated GeminiReviewer for now.
                let reviewer = GeminiReviewer;

                (Box::new(adapter), Arc::new(reviewer))
            }
        };

    // --- 6. Application Setup ---

    // This block sets up audio channels, gets an input device, configures it,
    // and prints the device information.
    // Audio channels for communication between tasks.
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Input>(1024);
    // Create the command channel to decouple core logic from the runtime.
    let (command_tx, mut command_rx) = tokio::sync::mpsc::channel::<feynman_core::Command>(32);

    // Setup audio input device.
    let input = feynman_native_utils::device::get_or_default_input(None)
        .context("Failed to get default audio input device")?;

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
    let output = feynman_native_utils::device::get_or_default_output(None)
        .context("Failed to get default audio output device")?;

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

    let topic = Topic {
        main_topic: args.topic,
    };

    tracing::info!(
        "Generating subtopics for main topic: '{}'",
        topic.main_topic
    );
    let subtopic_names = reviewer.generate_subtopics(&topic.main_topic).await?;
    let subtopics: Vec<SubTopic> = subtopic_names.into_iter().map(SubTopic::new).collect();
    let subtopic_list = SubTopicList::new(subtopics);
    tracing::debug!("Generated subtopics: {:?}", subtopic_list.subtopics);

    // Perform initial session configuration.
    let instructions = r#"You are a curious student in a Feynman session.
                        - You know ONLY what the teacher just said.
                        - When you receive one or more questions (one per line), read them and ask them out loud, one-by-one, in order.
                        - Do NOT answer questions or add information. Do NOT speculate or rephrase the teacher's content.
                        - If a single clarification question is received, just ask that one and stop.
                        - Keep each spoken question concise and natural."#;
    let session_config = GenericSessionConfig {
        instructions: instructions.to_string(),
    };
    realtime_api
        .update_session(session_config)
        .await
        .context("Failed to perform initial session update")?;
    tracing::info!("Initial session configuration sent.");

    // Create a resampler to configure the output sample rate.
    let mut out_resampler = feynman_native_utils::audio::create_resampler(
        REALTIME_API_PCM16_SAMPLE_RATE,
        output_sample_rate as f64,
        100,
    )?;

    // This channel receives base64 encoded audio from the server events task.
    let (_post_tx, mut post_rx) = tokio::sync::mpsc::channel::<Base64EncodedAudioBytes>(100);

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
        while let Some(e) = server_events.recv().await {
            // Match on the event type.
            match e {
                GenericServerEvent::Transcription(segment) => {
                    let segment = segment.trim().to_owned();
                    tracing::info!("User said: \"{}\"", segment);
                    FeynmanSession::process_segment(
                        &mut session,
                        &*reviewer2,
                        segment,
                        command_tx_for_server.clone(),
                    )
                    .await;
                }
                GenericServerEvent::Speaking => {
                    if let Err(e) = client_ctrl2.try_send(Input::AISpeaking()) {
                        tracing::warn!("Failed to send AISpeaking event to client: {:?}", e);
                    }
                }
                GenericServerEvent::SpeakingDone => {
                    if let Err(e) = client_ctrl2.try_send(Input::AISpeakingDone()) {
                        tracing::warn!("Failed to send AISpeakingDone event to client: {:?}", e);
                    }
                    tracing::debug!("Response done.");
                }
                GenericServerEvent::Error(err) => {
                    tracing::error!("Received server error: {}", err);
                }
                GenericServerEvent::Closed => {
                    tracing::info!("Connection closed by server.");
                    break;
                }
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
        let initialized = true; // Start as initialized now that config is sent upfront.
        let mut buffer: VecDeque<f32> = VecDeque::with_capacity(INPUT_CHUNK_SIZE * 2);

        // Receive and process inputs from the audio callbacks and server event handler.
        while let Some(i) = input_rx.recv().await {
            match i {
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
                        buffer.extend(audio);
                        let mut resampled: Vec<f32> = vec![];
                        while buffer.len() >= in_resampler.input_frames_next() {
                            let audio_chunk: Vec<f32> =
                                buffer.drain(..in_resampler.input_frames_next()).collect();

                            if let Ok(output_chunk) =
                                in_resampler.process(&[audio_chunk.as_slice()], None)
                            {
                                if let Some(channel_data) = output_chunk.first() {
                                    resampled.extend_from_slice(channel_data);
                                }
                            }
                        }
                        if !resampled.is_empty() {
                            let pcm16_audio =
                                feynman_native_utils::audio::convert_f32_to_i16(&resampled);
                            if let Err(e) =
                                realtime_api.append_input_audio_buffer(pcm16_audio).await
                            {
                                tracing::error!("Failed to send audio buffer: {:?}", e);
                            }
                        }
                    }
                }
                Input::CreateSpokenResponse(text) => {
                    if let Err(e) = realtime_api.create_spoken_response(text).await {
                        tracing::error!("Failed to create spoken response: {:?}", e);
                    }
                }
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

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use feynman_core::generic_types::GenericServerEvent;
    use mockall::mock;
    use rubato::ResampleError;

    /// A dummy resampler for testing purposes that correctly implements the Resampler trait.
    struct DummyResampler;
    impl Resampler<f32> for DummyResampler {
        fn process<V: AsRef<[f32]>>(
            &mut self,
            _waves_in: &[V],
            _active_channels_mask: Option<&[bool]>,
        ) -> Result<Vec<Vec<f32>>, ResampleError> {
            Ok(vec![vec![]])
        }

        fn process_into_buffer<Vin: AsRef<[f32]>, Vout: AsMut<[f32]>>(
            &mut self,
            _waves_in: &[Vin],
            waves_out: &mut [Vout],
            _active_channels_mask: Option<&[bool]>,
        ) -> Result<(usize, usize), ResampleError> {
            let nbr_frames = waves_out[0].as_mut().len();
            for chan in waves_out.iter_mut() {
                for sample in chan.as_mut().iter_mut() {
                    *sample = 0.0;
                }
            }
            Ok((0, nbr_frames))
        }

        fn input_frames_next(&self) -> usize {
            1024
        }
        fn input_frames_max(&self) -> usize {
            1024
        }
        fn output_frames_next(&self) -> usize {
            1024
        }
        fn output_frames_max(&self) -> usize {
            1024
        }
        fn nbr_channels(&self) -> usize {
            1
        }
        fn output_delay(&self) -> usize {
            0
        }
        fn set_resample_ratio(
            &mut self,
            _new_ratio: f64,
            _ramp: bool,
        ) -> Result<(), ResampleError> {
            Ok(())
        }
        fn set_resample_ratio_relative(
            &mut self,
            _ratio_factor: f64,
            _ramp: bool,
        ) -> Result<(), ResampleError> {
            Ok(())
        }
        fn reset(&mut self) {}
    }

    // Define the mock object for our new RealtimeApi trait
    mock! {
        pub RealtimeApi {}
        #[async_trait]
        impl RealtimeApi for RealtimeApi {
            async fn update_session(&mut self, config: GenericSessionConfig) -> Result<()>;
            async fn append_input_audio_buffer(&mut self, pcm_audio: Vec<i16>) -> Result<()>;
            async fn create_spoken_response(&mut self, text: String) -> Result<()>;
            async fn server_events(&mut self) -> Result<tokio::sync::mpsc::Receiver<GenericServerEvent>>;
        }
    }
}
