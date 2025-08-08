use crate::generic_types::{GenericServerEvent, GenericSessionConfig};
use anyhow::Result;
use async_trait::async_trait;

/// A trait abstracting a real-time, bidirectional AI service provider.
/// This allows the application to use different backends (like OpenAI or Gemini)
/// through a common interface.
#[async_trait]
pub trait RealtimeApi: Send + Sync {
    /// Initializes or updates the session with the provider.
    async fn update_session(&mut self, config: GenericSessionConfig) -> Result<()>;

    /// Appends a chunk of raw audio data (16-bit PCM) to the input buffer.
    async fn append_input_audio_buffer(&mut self, pcm_audio: Vec<i16>) -> Result<()>;

    /// Instructs the provider to speak a given text string.
    async fn create_spoken_response(&mut self, text: String) -> Result<()>;

    /// Returns a channel receiver for listening to server-side events.
    async fn server_events(&mut self) -> Result<tokio::sync::mpsc::Receiver<GenericServerEvent>>;
}
