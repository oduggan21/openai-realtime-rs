use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FrameCount, StreamConfig};
use feynman_core::{
    generic_types::{GenericServerEvent, GenericSessionConfig},
    realtime_api::RealtimeApi,
};

use feynman_native_utils::audio::REALTIME_API_PCM16_SAMPLE_RATE;
use feynman_native_utils::{audio, device};
use feynman_service::{gemini_adapter::GeminiAdapter, openai_adapter::OpenAIAdapter};
use ringbuf::traits::{Consumer, Producer, Split};
use rubato::Resampler;
use std::collections::VecDeque;
use std::env;
use tracing::Level;
use tracing_subscriber::fmt::time::ChronoLocal;

const INPUT_CHUNK_SIZE: usize = 1024;
const OUTPUT_CHUNK_SIZE: usize = 1024;
const OUTPUT_LATENCY_MS: usize = 1000;

pub enum Input {
    Audio(Vec<f32>),
    AISpeaking,
    AISpeakingDone,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from a .env file.
    dotenvy::dotenv().ok();

    // Create a tracing subscriber for tracking debug statements with timestamps.
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_timer(ChronoLocal::rfc_3339())
        .init();
    //-------------------------------------------------------------------------------/
    // This block sets up audio channels, gets an input device, configures it,
    // and prints the device information.
    // Audio channels for communication between tasks.
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Input>(1024);

    // Setup audio input device.
    let input = device::get_or_default_input(None).expect("failed to get input device");

    // Print the supported configs for the input device.
    println!("input: {:?}", &input.name().unwrap());
    input
        .supported_input_configs()
        .expect("failed to get supported input configs")
        .for_each(|c| println!("supported input config: {:?}", c));

    // Get the default input configuration.
    let input_config = input
        .default_input_config()
        .expect("failed to get default input config");

    // Create a stream config with a fixed buffer size.
    let input_config = StreamConfig {
        channels: input_config.channels(),
        sample_rate: input_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(INPUT_CHUNK_SIZE as u32)),
    };
    // Get the number of input channels.
    let input_channel_count = input_config.channels as usize;

    // Print the selected input device and its configuration.
    println!(
        "input: device={:?}, config={:?}",
        &input.name().unwrap(),
        &input_config
    );

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
            eprintln!("Failed to send audio data to buffer: {:?}", e);
        }
    };

    // Build the input stream.
    let input_stream = input
        .build_input_stream(
            &input_config,
            input_data_fn,
            move |err| eprintln!("an error occurred on input stream: {}", err),
            None,
        )
        .expect("failed to build input stream");

    input_stream.play().expect("failed to play input stream");
    let input_sample_rate = input_config.sample_rate.0 as f32;

    //------------------------------------------------------------/

    // Get the default output device.
    let output: cpal::Device =
        device::get_or_default_output(None).expect("failed to get output device");

    // Get the name of the output device.
    println!("output: {:?}", &output.name().unwrap());

    // Print the supported configs for the output device.
    output
        .supported_output_configs()
        .expect("failed to get supported output configs")
        .for_each(|c| println!("supported output config: {:?}", c));

    // Get the default output configuration.
    let output_config = output
        .default_output_config()
        .expect("failed to get default output config");
    // Create a stream config with a fixed buffer size.
    let output_config = StreamConfig {
        channels: output_config.channels(),
        sample_rate: output_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(FrameCount::from(OUTPUT_CHUNK_SIZE as u32)),
    };

    let output_channel_count = output_config.channels as usize;
    let output_sample_rate = output_config.sample_rate.0 as f32;
    println!(
        "output: device={:?}, config={:?}",
        &output.name().unwrap(),
        &output_config
    );

    let audio_out_buffer = audio::shared_buffer(output_sample_rate as usize * OUTPUT_LATENCY_MS);
    // Create a producer and consumer for the audio output buffer.
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
            if let Err(e) = client_ctrl.try_send(Input::AISpeakingDone) {
                eprintln!("Failed to send speaking done event to client: {:?}", e);
            }
        } else {
            if let Err(e) = client_ctrl.try_send(Input::AISpeaking) {
                eprintln!("Failed to send speaking event to client: {:?}", e);
            }
        }
    };
    // Build the output stream.
    let output_stream = output
        .build_output_stream(
            &output_config,
            output_data_fn,
            move |err| eprintln!("an error occurred on output stream: {}", err),
            None,
        )
        .expect("failed to build output stream");

    // Begin playing the output stream.
    output_stream.play().expect("failed to play output stream");

    // --- Realtime API ---
    let provider = env::var("REALTIME_PROVIDER").unwrap_or_else(|_| "openai".to_string());

    let mut realtime_api: Box<dyn RealtimeApi> = match provider.to_lowercase().as_str() {
        "gemini" => {
            println!("Using Gemini Provider");
            let api_key =
                env::var("GEMINI_API_KEY").context("GEMINI_API_KEY must be set for gemini")?;
            Box::new(GeminiAdapter::new(&api_key).await?)
        }
        _ => {
            println!("Using OpenAI Provider");
            let api_key =
                env::var("OPENAI_API_KEY").context("OPENAI_API_KEY must be set for openai")?;
            Box::new(OpenAIAdapter::new(api_key).await?)
        }
    };

    // Create a resampler to convert the API's audio sample rate to the output device's sample rate.
    let mut out_resampler = audio::create_resampler(
        REALTIME_API_PCM16_SAMPLE_RATE,
        output_sample_rate as f64,
        100,
    )
    .expect("failed to create resampler for output");

    // This channel receives base64 encoded audio from the server events task.
    let (_post_tx, mut post_rx) = tokio::sync::mpsc::channel::<String>(100);

    // This task receives audio from the server, decodes, resamples, and pushes it to the output buffer.
    let post_process = tokio::spawn(async move {
        // Receive audio from the server events task.
        while let Some(audio) = post_rx.recv().await {
            // Decode audio into a vector of floats.
            let audio_bytes = audio::decode(&audio);
            // Get the resampler's required chunk size.
            let chunk_size = out_resampler.input_frames_next();

            // Send the received audio to the audio buffer for playback.
            for samples in audio::split_for_chunks(&audio_bytes, chunk_size) {
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
    // Create a subscriber for server events.
    let mut server_events = realtime_api
        .server_events()
        .await
        .context("Failed to get server events channel")?;

    let server_handle = tokio::spawn(async move {
        // Receive and process events from the server.
        while let Some(e) = server_events.recv().await {
            match e {
                GenericServerEvent::Transcription(text) => {
                    println!("Human: {}", text.trim());
                }
                GenericServerEvent::Speaking => {
                    if let Err(e) = client_ctrl2.send(Input::AISpeaking).await {
                        eprintln!("Failed to send speaking event to client: {:?}", e);
                    }
                }
                GenericServerEvent::SpeakingDone => {
                    if let Err(e) = client_ctrl2.send(Input::AISpeakingDone).await {
                        eprintln!("Failed to send speaking done event to client: {:?}", e);
                    }
                }
                GenericServerEvent::Error(err) => {
                    eprintln!("Server Error: {}", err);
                }
                GenericServerEvent::Closed => {
                    println!("Connection closed.");
                    break;
                }
            }
        }
    });

    // Create a resampler to convert the input device's sample rate to the one required by the API.
    let mut in_resampler = audio::create_resampler(
        input_sample_rate as f64,
        REALTIME_API_PCM16_SAMPLE_RATE,
        INPUT_CHUNK_SIZE,
    )
    .expect("failed to create resampler for input");

    // This task handles client-side logic: sending user audio and managing state.
    let client_handle = tokio::spawn(async move {
        // Configure the session once at the start.
        let session_config = GenericSessionConfig {
            instructions: "You are a helpful and friendly voice assistant.".to_string(),
        };
        realtime_api
            .update_session(session_config)
            .await
            .expect("Failed to initialize session");
        println!("Session initialized.");

        let mut ai_speaking = false;
        let mut buffer: VecDeque<f32> = VecDeque::with_capacity(INPUT_CHUNK_SIZE * 2);

        // Receive and process inputs from the audio callbacks and server event handler.
        while let Some(i) = input_rx.recv().await {
            match i {
                Input::AISpeaking => {
                    if !ai_speaking {
                        println!("AI speaking...");
                    }
                    buffer.clear();
                    ai_speaking = true;
                }
                Input::AISpeakingDone => {
                    if ai_speaking {
                        println!("AI speaking done");
                    }
                    ai_speaking = false;
                }
                Input::Audio(audio) => {
                    if !ai_speaking {
                        buffer.extend(audio);
                        let mut resampled: Vec<f32> = vec![];
                        while buffer.len() >= INPUT_CHUNK_SIZE {
                            let audio_chunk: Vec<f32> = buffer.drain(..INPUT_CHUNK_SIZE).collect();
                            if let Ok(resamples) =
                                in_resampler.process(&[audio_chunk.as_slice()], None)
                            {
                                if let Some(resamples) = resamples.first() {
                                    resampled.extend(resamples.iter().cloned());
                                }
                            }
                        }
                        if resampled.is_empty() {
                            continue;
                        }
                        let pcm16_audio = audio::convert_f32_to_i16(&resampled);
                        realtime_api
                            .append_input_audio_buffer(pcm16_audio)
                            .await
                            .expect("failed to send audio");
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
    Ok(())
}
