// Outgoing messages
#[derive(serde::Serialize)]
pub struct ConfigRequest {
    pub config: SessionConfig,
}
#[derive(serde::Serialize)]
pub struct SessionConfig {
    pub instructions: String,
}

#[derive(serde::Serialize)]
pub struct TtsRequest {
    pub tts: TtsPayload,
}
#[derive(serde::Serialize)]
pub struct TtsPayload {
    pub text: String,
}

// Incoming messages
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ServerEvent {
    pub event: String,
    pub text: Option<String>,
}
