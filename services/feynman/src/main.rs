mod config;
mod prompt_loader;

use crate::config::{Config, INPUT_CHUNK_SIZE, OUTPUT_CHUNK_SIZE, OUTPUT_LATENCY_MS};
use anyhow::{Context, Result};
use async_trait::async_trait;
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
use rubato::{Resampler};
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
    /// This triggers a TTS synthesis and playback flow.
    CreateSpokenResponse(String),
}

#[derive(Parser)]
struct Cli {
    /// The main topic to teach
    topic: String,
}

/// A trait abstracting the `openai_realtime::Client` to allow for mocking in tests.
/// This defines the contract for the operations our application needs from the realtime API client.
#[async_trait]
pub trait RealtimeApi: Send {
    async fn update_session(&mut self, config: openai_realtime::types::Session) -> Result<()>;
    async fn append_input_audio_buffer(&mut self, audio: Base64EncodedAudioBytes) -> Result<()>;
    async fn create_conversation_item(&mut self, item: openai_realtime::types::Item) -> Result<()>;
    async fn create_response(&mut self) -> Result<()>;
    async fn server_events(&mut self) -> Result<openai_realtime::ServerRx>;
}

/// Implements the `RealtimeApi` trait for the actual `openai_realtime::Client`.
/// This implementation simply delegates the calls to the real client.
#[async_trait]
impl RealtimeApi for openai_realtime::Client {
    async fn update_session(&mut self, config: openai_realtime::types::Session) -> Result<()> {
        self.update_session(config).await
    }
    async fn append_input_audio_buffer(&mut self, audio: Base64EncodedAudioBytes) -> Result<()> {
        self.append_input_audio_buffer(audio).await
    }
    async fn create_conversation_item(&mut self, item: openai_realtime::types::Item) -> Result<()> {
        self.create_conversation_item(item).await
    }
    async fn create_response(&mut self) -> Result<()> {
        self.create_response().await
    }
    async fn server_events(&mut self) -> Result<openai_realtime::ServerRx> {
        self.server_events().await
    }
}

/// Manages the state and logic for interacting with the OpenAI Realtime API.
/// This struct encapsulates the client-side logic, making it testable and easier to reason about.
struct ClientHandler<T: RealtimeApi, R: Resampler<f32> + Send> {
    realtime_api: T,
    ai_speaking: bool,
    initialized: bool,
    buffer: VecDeque<f32>,
    in_resampler: R,
}

impl<T: RealtimeApi, R: Resampler<f32> + Send> ClientHandler<T, R> {
    /// Processes a single `Input` event, updating state and interacting with the Realtime API.
    /// This function contains the core client-side logic for handling audio, state changes, and commands.
    async fn handle_input(&mut self, i: Input) -> Result<()> {
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
                self.realtime_api
                    .update_session(session)
                    .await
                    .context("Failed to initialize session")?;
            }
            Input::Initialized() => {
                tracing::info!("Session initialized successfully.");
                self.initialized = true;
            }
            Input::AISpeaking() => {
                if !self.ai_speaking {
                    tracing::debug!("AI speaking...");
                }
                self.buffer.clear();
                self.ai_speaking = true;
            }
            Input::AISpeakingDone() => {
                if self.ai_speaking {
                    tracing::debug!("AI speaking done");
                }
                self.ai_speaking = false;
            }
            Input::Audio(audio) => {
                if self.initialized && !self.ai_speaking {
                    self.buffer.extend(audio);
                    let mut resampled: Vec<f32> = vec![];
                    while self.buffer.len() >= self.in_resampler.input_frames_next() {
                        let audio_chunk: Vec<f32> =
                            self.buffer.drain(..self.in_resampler.input_frames_next()).collect();

                        // The `process` method on the trait returns a new Vec, which is less efficient
                        // but simpler to use here. The real `FastFixedIn` also has this method.
                        if let Ok(output_chunk) =
                            self.in_resampler.process(&[audio_chunk.as_slice()], None)
                        {
                            if let Some(channel_data) = output_chunk.first() {
                                resampled.extend_from_slice(channel_data);
                            }
                        }
                    }
                    if !resampled.is_empty() {
                        let audio_bytes = feynman_native_utils::audio::encode(&resampled);
                        let audio_bytes = Base64EncodedAudioBytes::from(audio_bytes);
                        self.realtime_api
                            .append_input_audio_buffer(audio_bytes.clone())
                            .await
                            .context("Failed to send audio buffer")?;
                    }
                }
            }
            // Handles the command to make the AI speak.
            Input::CreateSpokenResponse(text) => {
                // This is a two-step process to make the AI speak on demand:
                // 1. Create a "system" message containing the text we want the AI to say.
                //    This injects the text into the conversation history.
                let item = openai_realtime::types::MessageItem::builder()
                    .with_role(openai_realtime::types::MessageRole::System)
                    .with_input_text(&text)
                    .build();

                self.realtime_api
                    .create_conversation_item(openai_realtime::types::Item::Message(item))
                    .await
                    .context("Failed to create conversation item for AI speech")?;

                // 2. Trigger a response. The API will see the last message (the one we just sent)
                //    and generate audio for it, effectively making the AI speak our provided text.
                self.realtime_api
                    .create_response()
                    .await
                    .context("Failed to trigger response for AI speech")?;
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Load Configuration ---
    let config = Config::from_env().context("Failed to load application configuration")?;
    // --- 1.B 

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
     let prompts_path = Path::new(manifest_dir).join("prompts");
    // --- 2. Initialize Logging ---
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_timer(ChronoLocal::rfc_3339())
        .init();

    tracing::info!("Configuration loaded successfully. Starting Feynman service...");

    // --- 3. Parse Command-Line Arguments ---
    let args = Cli::parse();

    // --- 4. Load Prompts ---
    let prompts = prompt_loader::load_prompts(&prompts_path)
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
            for samples in feynman_native_utils::audio::split_for_chunks(&audio_bytes, chunk_size)
            {
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
    let in_resampler = feynman_native_utils::audio::create_resampler(
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
        let mut handler = ClientHandler {
            realtime_api,
            ai_speaking: false,
            initialized: false,
            buffer: VecDeque::with_capacity(INPUT_CHUNK_SIZE * 2),
            in_resampler,
        };

        // Receive and process inputs from the audio callbacks and server event handler.
        while let Some(i) = input_rx.recv().await {
            if let Err(e) = handler.handle_input(i).await {
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

#[cfg(test)]
mod tests {
    use super::*;
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
        fn set_resample_ratio(&mut self, _new_ratio: f64, _ramp: bool) -> Result<(), ResampleError> {
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

    // Define the mock object for our RealtimeApi trait
    mock! {
        pub RealtimeApi {}
        #[async_trait]
        impl RealtimeApi for RealtimeApi {
            async fn update_session(&mut self, config: openai_realtime::types::Session) -> Result<()>;
            async fn append_input_audio_buffer(&mut self, audio: Base64EncodedAudioBytes) -> Result<()>;
            async fn create_conversation_item(&mut self, item: openai_realtime::types::Item) -> Result<()>;
            async fn create_response(&mut self) -> Result<()>;
            async fn server_events(&mut self) -> Result<openai_realtime::ServerRx>;
        }
    }

    #[tokio::test]
    async fn test_handle_input_create_spoken_response() {
        // --- Arrange ---
        let mut mock_api = MockRealtimeApi::new();
        let question_text = "What is the meaning of life?".to_string();

        // Set up expectations on the mock API.
        // We expect `create_conversation_item` to be called once with a specific payload.
        mock_api
            .expect_create_conversation_item()
            .withf(move |item| {
                if let openai_realtime::types::Item::Message(msg) = item {
                    if msg.role() == openai_realtime::types::MessageRole::System {
                        if let Some(openai_realtime::types::Content::InputText(content)) =
                            msg.content().get(0)
                        {
                            return content.text() == question_text;
                        }
                    }
                }
                false
            })
            .times(1)
            .returning(|_| Ok(()));

        // We expect `create_response` to be called once, after the item is created.
        mock_api
            .expect_create_response()
            .times(1)
            .returning(|| Ok(()));

        let mut handler = ClientHandler {
            realtime_api: mock_api,
            ai_speaking: false,
            initialized: true,
            buffer: VecDeque::new(),
            in_resampler: DummyResampler,
        };

        let input = Input::CreateSpokenResponse("What is the meaning of life?".to_string());

        // --- Act ---
        let result = handler.handle_input(input).await;

        // --- Assert ---
        assert!(result.is_ok());
        // The mock assertions automatically verify that the expected calls were made.
    }
}