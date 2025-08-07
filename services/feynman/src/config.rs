//! Application Configuration Module
//!
//! This module centralizes the configuration for the Feynman service.
//! It loads settings from environment variables and provides a single,
//! shareable struct that can be passed throughout the application.

use std::env;
use tracing::Level;

// --- Application Constants ---

/// The size of each audio chunk sent from the microphone input stream.
pub const INPUT_CHUNK_SIZE: usize = 1024;
/// The size of each audio chunk for the audio output stream.
pub const OUTPUT_CHUNK_SIZE: usize = 1024;
/// The latency for the output audio buffer in milliseconds.
pub const OUTPUT_LATENCY_MS: usize = 1000;

/// Holds all configuration loaded from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub openai_api_key: String,
    pub chat_model: String,
    pub log_level: Level,
}

/// A custom error type for configuration loading failures.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingVar(String),
    #[error("Invalid log level provided for RUST_LOG: {0}")]
    InvalidLogLevel(String),
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    // *   `OPENAI_API_KEY`: Your secret key for the OpenAI API.
    // *   `CHAT_MODEL`: (Optional) The model to use for the Reviewer AI. Defaults to "gpt-4o".
    // *   `RUST_LOG`: (Optional) The logging level. Defaults to "INFO". Can be "TRACE", "DEBUG", "INFO", "WARN", or "ERROR".
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file. This is useful for local development and is ignored if not present.
        dotenvy::dotenv().ok();

        let openai_api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ConfigError::MissingVar("OPENAI_API_KEY".to_string()))?;

        // Provide a default for non-critical variables.
        let chat_model = env::var("CHAT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        // Configure logging level from RUST_LOG, with a sensible default.
        let log_level_str = env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());
        let log_level = log_level_str
            .parse::<Level>()
            .map_err(|_| ConfigError::InvalidLogLevel(log_level_str))?;

        Ok(Self {
            openai_api_key,
            chat_model,
            log_level,
        })
    }
}
