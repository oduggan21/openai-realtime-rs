use anyhow::Result;
use async_trait::async_trait;

mod client;

pub use openai_realtime_types as types;

pub use client::{Client, ServerRx, connect};
pub use types::{Item, Session};

/// A trait that abstracts the `openai_realtime::Client` to allow for mocking in tests.
/// This defines the contract for the operations our adapter needs from the underlying client.
#[async_trait]
pub trait OAIClient: Send + Sync {
    async fn update_session(&mut self, config: Session) -> Result<()>;
    async fn append_input_audio_buffer(&mut self, audio: String) -> Result<()>;
    async fn create_conversation_item(&mut self, item: Item) -> Result<()>;
    async fn create_response(&mut self) -> Result<()>;
    async fn server_events(&mut self) -> Result<ServerRx>;
}

/// Implements the `OAIClient` trait for the actual `openai_realtime::Client`.
/// This implementation simply delegates the calls to the real client.
#[async_trait]
impl OAIClient for crate::Client {
    async fn update_session(&mut self, config: Session) -> Result<()> {
        self.update_session(config).await
    }
    async fn append_input_audio_buffer(&mut self, audio: String) -> Result<()> {
        self.append_input_audio_buffer(audio).await
    }
    async fn create_conversation_item(&mut self, item: Item) -> Result<()> {
        self.create_conversation_item(item).await
    }
    async fn create_response(&mut self) -> Result<()> {
        self.create_response().await
    }
    async fn server_events(&mut self) -> Result<ServerRx> {
        self.server_events().await
    }
}
