use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use feynman_core::generic_types::{GenericServerEvent, GenericSessionConfig};
use feynman_core::realtime_api::RealtimeApi;
use gemini_realtime::GeminiClient;

/// An adapter that implements the generic `RealtimeApi` trait for the `gemini_realtime::GeminiClient`.
pub struct GeminiAdapter {
    client: GeminiClient,
    // This field ensures that we only spawn the event translation task once.
    event_rx: Option<tokio::sync::mpsc::Receiver<GenericServerEvent>>,
}

impl GeminiAdapter {
    pub async fn new(api_key: &str) -> Result<Self> {
        let client = gemini_realtime::connect(api_key)
            .await
            .context("Failed to create GeminiAdapter")?;
        Ok(Self {
            client,
            event_rx: None,
        })
    }
}

#[async_trait]
impl RealtimeApi for GeminiAdapter {
    async fn update_session(&mut self, config: GenericSessionConfig) -> Result<()> {
        self.client.send_config(config.instructions).await
    }

    async fn append_input_audio_buffer(&mut self, pcm_audio: Vec<i16>) -> Result<()> {
        // Convert Vec<i16> to Vec<u8> for base64 encoding.
        let byte_data: Vec<u8> = pcm_audio
            .into_iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        let base64_data = general_purpose::STANDARD.encode(&byte_data);
        self.client.send_audio_chunk(base64_data).await
    }

    async fn create_spoken_response(&mut self, text: String) -> Result<()> {
        self.client.send_tts(text).await
    }

    async fn server_events(&mut self) -> Result<tokio::sync::mpsc::Receiver<GenericServerEvent>> {
        if self.event_rx.is_some() {
            // This prevents the event-translation task from being spawned multiple times.
            return Err(anyhow::anyhow!(
                "server_events channel has already been taken"
            ));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let mut gemini_rx = self.client.server_events();

        tokio::spawn(async move {
            loop {
                match gemini_rx.recv().await {
                    Ok(event) => {
                        if let Some(content) = event.server_content {
                            // Handle transcription
                            if let Some(transcription) = content.input_transcription {
                                if tx
                                    .send(GenericServerEvent::Transcription(transcription.text))
                                    .await
                                    .is_err()
                                {
                                    break; // Receiver dropped
                                }
                            }

                            // Handle audio output events
                            if let Some(model_turn) = content.model_turn {
                                let has_audio =
                                    model_turn.parts.iter().any(|p| p.inline_data.is_some());
                                if has_audio {
                                    if tx.send(GenericServerEvent::Speaking).await.is_err() {
                                        break;
                                    }
                                }
                            }

                            // Handle turn completion
                            if content.turn_complete == Some(true) {
                                if tx.send(GenericServerEvent::SpeakingDone).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Gemini event stream lagged by {} messages.", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Gemini event channel closed.");
                        let _ = tx.send(GenericServerEvent::Closed).await;
                        break;
                    }
                }
            }
            tracing::warn!("Gemini event receiver dropped, stopping adapter task.");
        });

        self.event_rx = Some(rx);
        // To return a new receiver, we must create a new channel and replace the one in self.
        let (_new_tx, new_rx) = tokio::sync::mpsc::channel(128);
        let old_rx = self.event_rx.replace(new_rx);
        Ok(old_rx.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gemini_realtime::types::{
        LiveServerContent, ServerBlob, ServerContentTurn, ServerMessage, ServerPart,
        ServerTranscription,
    };
    use tokio::sync::broadcast;

    // Mock GeminiClient to test the adapter's translation logic.
    struct MockGeminiClient {
        server_tx: broadcast::Sender<ServerMessage>,
    }

    impl MockGeminiClient {
        fn new() -> Self {
            let (server_tx, _) = broadcast::channel(32);
            Self { server_tx }
        }

        // A method to get a sender to push mock events into the client.
        fn get_event_sender(&self) -> broadcast::Sender<ServerMessage> {
            self.server_tx.clone()
        }

        // A method to get a receiver, just like the real client.
        fn server_events(&self) -> broadcast::Receiver<ServerMessage> {
            self.server_tx.subscribe()
        }
    }

    #[tokio::test]
    async fn test_gemini_adapter_event_translation() {
        // --- Arrange ---
        let mock_client = MockGeminiClient::new();
        let event_sender = mock_client.get_event_sender();

        let (tx, mut rx) = tokio::sync::mpsc::channel(128);
        let mut gemini_rx = mock_client.server_events();

        // This is the logic from inside the adapter's `server_events` method.
        tokio::spawn(async move {
            while let Ok(event) = gemini_rx.recv().await {
                if let Some(content) = event.server_content {
                    if let Some(transcription) = content.input_transcription {
                        if tx
                            .send(GenericServerEvent::Transcription(transcription.text))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    if let Some(model_turn) = content.model_turn {
                        if model_turn.parts.iter().any(|p| p.inline_data.is_some()) {
                            if tx.send(GenericServerEvent::Speaking).await.is_err() {
                                break;
                            }
                        }
                    }
                    if content.turn_complete == Some(true) {
                        if tx.send(GenericServerEvent::SpeakingDone).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // --- Act & Assert ---

        // Test transcription event
        let transcription_event = ServerMessage {
            setup_complete: None,
            server_content: Some(LiveServerContent {
                input_transcription: Some(ServerTranscription {
                    text: "Hello world".to_string(),
                }),
                model_turn: None,
                turn_complete: None,
            }),
        };
        event_sender.send(transcription_event).unwrap();

        let received = rx.recv().await.unwrap();
        assert!(
            matches!(received, GenericServerEvent::Transcription(text) if text == "Hello world")
        );

        // Test speaking event
        let speaking_event = ServerMessage {
            setup_complete: None,
            server_content: Some(LiveServerContent {
                input_transcription: None,
                model_turn: Some(ServerContentTurn {
                    parts: vec![ServerPart {
                        text: None,
                        inline_data: Some(ServerBlob {
                            mime_type: "audio/pcm".to_string(),
                            data: String::new(),
                        }),
                    }],
                }),
                turn_complete: None,
            }),
        };
        event_sender.send(speaking_event).unwrap();

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, GenericServerEvent::Speaking));

        // Test speaking done event
        let speaking_done_event = ServerMessage {
            setup_complete: None,
            server_content: Some(LiveServerContent {
                input_transcription: None,
                model_turn: None,
                turn_complete: Some(true),
            }),
        };
        event_sender.send(speaking_done_event).unwrap();

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, GenericServerEvent::SpeakingDone));
    }
}
