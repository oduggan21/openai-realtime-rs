use std::net::SocketAddr;
use tracing::Level;

/// A custom error type for configuration loading failures.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingVar(String),
    #[error("Invalid value for environment variable {0}: {1}")]
    InvalidValue(String, String),
}

/// Holds all configuration loaded from the environment at startup.
#[derive(Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub openai_api_key: String,
    pub chat_model: String,
    pub log_level: Level,
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// This function will look for a `.env` file in the current directory
    /// and load the following variables:
    ///
    /// *   `BIND_ADDRESS`: The address and port to bind the server to (e.g., "0.0.0.0:3000").
    /// *   `OPENAI_API_KEY`: Your secret key for the OpenAI API.
    /// *   `CHAT_MODEL`: (Optional) The model to use for the Reviewer AI. Defaults to "gpt-4o".
    /// *   `RUST_LOG`: (Optional) The logging level. Defaults to "INFO".
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let bind_address_str =
            std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        // Add type annotation to `parse` to help the compiler infer the error type.
        let bind_address = bind_address_str
            .parse::<SocketAddr>()
            .map_err(|e| ConfigError::InvalidValue("BIND_ADDRESS".to_string(), e.to_string()))?;

        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ConfigError::MissingVar("OPENAI_API_KEY".to_string()))?;

        let chat_model = std::env::var("CHAT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        let log_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());
        let log_level = log_level_str.parse::<Level>().map_err(|_| {
            ConfigError::InvalidValue(
                "RUST_LOG".to_string(),
                format!("'{}' is not a valid log level", log_level_str),
            )
        })?;

        Ok(Self {
            bind_address,
            openai_api_key,
            chat_model,
            log_level,
        })
    }
}
