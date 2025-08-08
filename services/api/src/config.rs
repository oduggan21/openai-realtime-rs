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

/// Defines the supported backend providers for the Reviewer service.
#[derive(Clone, Debug)]
pub enum Provider {
    OpenAI,
    Gemini,
}

/// Holds all configuration loaded from the environment at startup.
#[derive(Clone)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub provider: Provider,
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
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
    /// *   `REALTIME_PROVIDER`: The backend to use. Can be "openai" or "gemini". Defaults to "openai".
    /// *   `OPENAI_API_KEY`: Your secret key for the OpenAI API. Required if provider is "openai".
    /// *   `GEMINI_API_KEY`: Your secret key for the Gemini API. Required if provider is "gemini".
    /// *   `CHAT_MODEL`: (Optional) The model to use for the Reviewer AI. Defaults to "gpt-4o".
    /// *   `RUST_LOG`: (Optional) The logging level. Defaults to "INFO".
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let bind_address_str =
            std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let bind_address = bind_address_str
            .parse::<SocketAddr>()
            .map_err(|e| ConfigError::InvalidValue("BIND_ADDRESS".to_string(), e.to_string()))?;

        let provider_str =
            std::env::var("REALTIME_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let provider = match provider_str.to_lowercase().as_str() {
            "gemini" => Provider::Gemini,
            _ => Provider::OpenAI,
        };

        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();

        let chat_model = std::env::var("CHAT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        let log_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());
        let log_level = log_level_str.parse::<Level>().map_err(|_| {
            ConfigError::InvalidValue(
                "RUST_LOG".to_string(),
                format!("'{}' is not a valid log level", log_level_str),
            )
        })?;

        // Validate that the required API key is present for the selected provider.
        match provider {
            Provider::OpenAI => {
                if openai_api_key.is_none() {
                    return Err(ConfigError::MissingVar(
                        "OPENAI_API_KEY must be set for 'openai' provider".to_string(),
                    ));
                }
            }
            Provider::Gemini => {
                if gemini_api_key.is_none() {
                    return Err(ConfigError::MissingVar(
                        "GEMINI_API_KEY must be set for 'gemini' provider".to_string(),
                    ));
                }
            }
        }

        Ok(Self {
            bind_address,
            provider,
            openai_api_key,
            gemini_api_key,
            chat_model,
            log_level,
        })
    }
}
