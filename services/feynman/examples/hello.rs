use anyhow::{Context, Result};
use feynman_core::{
    generic_types::{GenericServerEvent, GenericSessionConfig},
    realtime_api::RealtimeApi,
};
use feynman_service::{gemini_adapter::GeminiAdapter, openai_adapter::OpenAIAdapter};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Setup ---
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // --- 2. Dynamic Provider Selection ---
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

    // --- 3. Get Server Events Stream ---
    let mut server_events = realtime_api
        .server_events()
        .await
        .context("Failed to get server events channel")?;

    println!("Connected to Realtime API via adapter.");

    // --- 4. Spawn a task to listen to and print all events ---
    tokio::spawn(async move {
        while let Some(event) = server_events.recv().await {
            match event {
                GenericServerEvent::Transcription(text) => {
                    println!("USER SAID: {}", text);
                }
                GenericServerEvent::Speaking => {
                    println!("AI is speaking...");
                }
                GenericServerEvent::SpeakingDone => {
                    println!("AI finished speaking.");
                }
                GenericServerEvent::Error(e) => {
                    eprintln!("Received server error: {}", e);
                }
                GenericServerEvent::Closed => {
                    println!("Connection closed.");
                    break;
                }
            }
        }
    });

    // --- 5. Configure and start the session ---
    let session_config = GenericSessionConfig {
        instructions: "You are a helpful assistant.".to_string(),
    };
    realtime_api
        .update_session(session_config)
        .await
        .context("Failed to update session")?;

    println!("Session configured. Sending a TTS request...");

    // --- 6. Make the AI speak ---
    realtime_api
        .create_spoken_response("Hello from the generic Realtime API trait!".to_string())
        .await
        .context("Failed to create spoken response")?;

    // --- 7. Wait to observe events ---
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    Ok(())
}
