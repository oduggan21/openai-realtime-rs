mod config;
mod gemini_adapter;
mod openai_adapter;
mod prompt_loader;

use crate::config::{Config, RealtimeProvider};
use anyhow::{Context, Result};
use clap::Parser;
use feynman_core::agent::{FeynmanAgent, FeynmanService};
use feynman_core::gemini_reviewer::GeminiReviewer;
use feynman_core::reviewer::{OpenAIReviewer, Reviewer};
use feynman_core::topic::{SubTopic, SubTopicList};
use rmcp::ServiceExt;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::fmt::time::ChronoLocal;

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
    let main_topic = args.topic;

    // --- 4. Load Prompts ---
    let prompts =
        prompt_loader::load_prompts(Path::new("prompts")).context("Failed to load LLM prompts")?;
    tracing::info!("Loaded {} prompts successfully.", prompts.len());

    // --- 5. Initialize Reviewer Client ---
    // The reviewer is responsible for all LLM calls for analysis and generation.
    let reviewer: Arc<dyn Reviewer> = match config.provider {
        RealtimeProvider::OpenAI => {
            tracing::info!("Using OpenAI Provider for Reviewer");
            let api_key = config
                .openai_api_key
                .context("OPENAI_API_KEY must be set for openai provider")?;
            Arc::new(OpenAIReviewer::new(
                api_key,
                config.chat_model.clone(),
                prompts,
            ))
        }
        RealtimeProvider::Gemini => {
            tracing::info!("Using simulated Gemini Provider for Reviewer");
            // Using the simulated GeminiReviewer for now.
            Arc::new(GeminiReviewer)
        }
    };

    // --- 6. Setup Feynman Agent ---
    tracing::info!("Generating subtopics for main topic: '{}'", main_topic);
    let subtopic_names = reviewer.generate_subtopics(&main_topic).await?;
    let subtopics: Vec<SubTopic> = subtopic_names.into_iter().map(SubTopic::new).collect();
    let subtopic_list = SubTopicList::new(subtopics);
    tracing::debug!("Generated subtopics: {:?}", subtopic_list.subtopics);

    // Create the agent's state, wrapping it for concurrent access.
    let agent_state = Arc::new(tokio::sync::Mutex::new(FeynmanAgent::new(
        main_topic,
        subtopic_list,
    )));

    // --- 7. Initialize and Run the MCP Service ---
    // The FeynmanService bundles the state and dependencies into an MCP-compliant handler.
    let feynman_service = FeynmanService::new(agent_state, reviewer);

    tracing::info!("Starting Feynman MCP agent over stdio...");

    // Create the MCP service using a standard I/O transport.
    // This allows the agent to communicate with a client (like an inspector) over stdin/stdout.
    let service = feynman_service
        .serve(rmcp::transport::stdio())
        .await
        .context("Failed to start MCP service")?;

    // Wait for the service to complete or be cancelled.
    service
        .waiting()
        .await
        .context("MCP service event loop failed")?;

    tracing::info!("Shutting down...");
    Ok(())
}
