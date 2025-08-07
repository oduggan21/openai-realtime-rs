use secrecy::SecretString;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;

pub struct Config {
    base_url: String,
    api_key: SecretString,
    model: String,
}

pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.config.base_url = base_url.to_string();
        self
    }

    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.config.api_key = SecretString::from(api_key.to_string());
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.config.model = model.to_string();
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

impl Config {
    // Sets the default values.
    pub fn new() -> Self {
        Self {
            // Convert the &str to String.
            base_url: "wss://api.openai.com/v1".to_string(),
            // Get the API key from the environment variable.
            // Unwrap the Result and handle it so we don't panic.
            // Convert api_key to SecretString if necessary.
            api_key: std::env::var("OPENAI_API_KEY")
                .unwrap_or_else(|_| "".to_string())
                .into(),
            // Declare the default model.
            model: "gpt-4o-realtime-preview-2024-10-01".to_string(),
        }
    }

    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn api_key(&self) -> &SecretString {
        &self.api_key
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}
