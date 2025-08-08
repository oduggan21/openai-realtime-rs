use anyhow::{Context, Result};
use async_trait::async_trait;
use feynman_core::generic_types::{GenericServerEvent, GenericSessionConfig};
use feynman_core::realtime_api::RealtimeApi;
use feynman_native_utils::audio;
use openai_realtime::OAIClient;
use openai_realtime::types::audio::{
    ServerVadTurnDetection, TranscriptionModel, TurnDetection, Voice,
};

/// An adapter that implements the generic `RealtimeApi` trait for the `openai_realtime::Client`.
/// It is generic over a trait `OAIClient` to allow for mocking the underlying client in tests.
pub struct OpenAIAdapter<C: OAIClient> {
    client: C,
    event_rx: Option<tokio::sync::mpsc::Receiver<GenericServerEvent>>,
}

impl OpenAIAdapter<openai_realtime::Client> {
    pub async fn new() -> Result<Self> {
        let client = openai_realtime::connect()
            .await
            .context("Failed to connect to OpenAI Realtime API")?;
        Ok(Self {
            client,
            event_rx: None,
        })
    }
}

#[async_trait]
impl<C: OAIClient> RealtimeApi for OpenAIAdapter<C> {
    async fn update_session(&mut self, config: GenericSessionConfig) -> Result<()> {
        let turn_detection = TurnDetection::ServerVad(
            ServerVadTurnDetection::default()
                .with_interrupt_response(true)
                .with_create_response(false),
        );

        let session = openai_realtime::types::Session::new()
            .with_modalities_enable_audio()
            .with_instructions(&config.instructions)
            .with_voice(Voice::Alloy)
            .with_input_audio_transcription_enable(TranscriptionModel::Whisper)
            .with_turn_detection_enable(turn_detection)
            .build();

        self.client.update_session(session).await
    }

    async fn append_input_audio_buffer(&mut self, pcm_audio: Vec<i16>) -> Result<()> {
        let encoded_audio = audio::encode_i16(&pcm_audio);
        self.client.append_input_audio_buffer(encoded_audio).await
    }

    async fn create_spoken_response(&mut self, text: String) -> Result<()> {
        let item = openai_realtime::types::MessageItem::builder()
            .with_role(openai_realtime::types::MessageRole::System)
            .with_input_text(&text)
            .build();

        self.client
            .create_conversation_item(openai_realtime::types::Item::Message(item))
            .await
            .context("Adapter failed to create conversation item for AI speech")?;

        self.client
            .create_response()
            .await
            .context("Adapter failed to trigger response for AI speech")?;
        Ok(())
    }

    async fn server_events(&mut self) -> Result<tokio::sync::mpsc::Receiver<GenericServerEvent>> {
        if self.event_rx.is_some() {
            return Err(anyhow::anyhow!(
                "server_events channel has already been taken"
            ));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let mut openai_rx = self.client.server_events().await?;

        tokio::spawn(async move {
            while let Ok(event) = openai_rx.recv().await {
                let generic_event = match event {
                    openai_realtime::types::events::ServerEvent::ConversationItemInputAudioTranscriptionCompleted(data) => {
                        Some(GenericServerEvent::Transcription(data.transcript().to_string()))
                    },
                    // You can add more detailed events here if needed, like SpeakingStarted/Stopped
                    openai_realtime::types::events::ServerEvent::ResponseAudioDelta(_) => {
                        Some(GenericServerEvent::Speaking)
                    }
                    openai_realtime::types::events::ServerEvent::ResponseDone(_) => {
                        Some(GenericServerEvent::SpeakingDone)
                    }
                    openai_realtime::types::events::ServerEvent::Error(e) => {
                        Some(GenericServerEvent::Error(e.error().message().to_string()))
                    }
                    openai_realtime::types::events::ServerEvent::Close {..} => {
                        Some(GenericServerEvent::Closed)
                    }
                    _ => None, // Ignore other event types
                };

                if let Some(ge) = generic_event {
                    if tx.send(ge).await.is_err() {
                        tracing::warn!("Generic event receiver dropped, stopping adapter task.");
                        break;
                    }
                }
            }
        });

        self.event_rx = Some(rx);
        // We need a new receiver for the caller, so we create a new channel and replace the one in self.
        let (_new_tx, new_rx) = tokio::sync::mpsc::channel(128);
        let old_rx = self.event_rx.replace(new_rx);
        Ok(old_rx.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate::*};
    use openai_realtime::OAIClient;
    use openai_realtime::types::{Item, MessageRole};

    mock! {
        pub OAIClient {}
        #[async_trait]
        impl OAIClient for OAIClient {
            async fn update_session(&mut self, config: openai_realtime::types::Session) -> Result<()>;
            async fn append_input_audio_buffer(&mut self, audio: String) -> Result<()>;
            async fn create_conversation_item(&mut self, item: Item) -> Result<()>;
            async fn create_response(&mut self) -> Result<()>;
            async fn server_events(&mut self) -> Result<openai_realtime::ServerRx>;
        }
    }

    #[tokio::test]
    async fn test_create_spoken_response() {
        // --- Arrange ---
        let mut mock_client = MockOAIClient::new();
        let question_text = "What is the meaning of life?".to_string();

        // Set up expectations on the mock client.
        // We expect `create_conversation_item` to be called once.
        mock_client
            .expect_create_conversation_item()
            .withf(move |item| {
                // Use `withf` to inspect the argument
                if let Item::Message(msg) = item {
                    if msg.role() == MessageRole::System {
                        if let Some(openai_realtime::types::Content::InputText(content)) =
                            msg.content().get(0)
                        {
                            return content.text() == question_text;
                        }
                    }
                }
                false
            })
            .times(1)
            .returning(|_| Ok(()));

        // We expect `create_response` to be called once, after the item is created.
        mock_client
            .expect_create_response()
            .times(1)
            .returning(|| Ok(()));

        let mut adapter = OpenAIAdapter {
            client: mock_client,
            event_rx: None,
        };

        // --- Act ---
        let result = adapter
            .create_spoken_response("What is the meaning of life?".to_string())
            .await;

        // --- Assert ---
        assert!(result.is_ok());
        // The mock's expectations are automatically verified when `adapter` is dropped.
    }
}
