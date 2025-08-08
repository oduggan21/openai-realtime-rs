/// Generic configuration for initializing a real-time session with any provider.
#[derive(Debug, Clone, Default)]
pub struct GenericSessionConfig {
    pub instructions: String,
    // Add other common fields here in the future if needed.
}

/// Generic events that any real-time provider can emit back to the application.
#[derive(Debug, Clone)]
pub enum GenericServerEvent {
    Transcription(String),
    Speaking,
    SpeakingDone,
    Error(String),
    Closed,
}
