use crate::types::{
    BidiGenerateContentClientContent, BidiGenerateContentRealtimeInput, BidiGenerateContentSetup,
    Blob, ClientMessage, Content, GenerationConfig, Part, ResponseModality, ServerMessage,
};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::protocol::Message;

/// A client for the Gemini Real-time WebSocket API.
/// It manages its own internal read/write tasks.
pub struct GeminiClient {
    client_tx: mpsc::Sender<Message>,
    server_tx: broadcast::Sender<ServerMessage>,
}

/// Establishes a connection to the Gemini real-time service.
pub async fn connect(api_key: &str) -> Result<GeminiClient> {
    let url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key={}",
        api_key
    );
    let (ws_stream, _) = tokio_tungstenite::connect_async(url)
        .await
        .context("Failed to connect to Gemini WebSocket")?;

    tracing::info!("Successfully connected to Gemini WebSocket.");
    let (mut write, mut read) = ws_stream.split();

    let (client_tx, mut client_rx) = mpsc::channel::<Message>(32);
    let (server_tx, _) = broadcast::channel::<ServerMessage>(32);
    let server_tx_clone = server_tx.clone();

    // Write task: reads from MPSC and sends to WebSocket
    tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            if write.send(msg).await.is_err() {
                tracing::error!("Failed to send message to Gemini WebSocket; connection closed.");
                break;
            }
        }
    });

    // Read task: reads from WebSocket and broadcasts events
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(event) => {
                            if server_tx_clone.send(event).is_err() {
                                // This is not an error, just means the main app is no longer listening.
                                tracing::info!(
                                    "No active receivers for Gemini server events; stopping read task."
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to deserialize Gemini server event: {}. Raw text: {}",
                                e,
                                text
                            );
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    tracing::warn!("Received unexpected binary message from Gemini server.");
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Gemini WebSocket connection closed by server.");
                    break;
                }
                Err(e) => {
                    tracing::error!("Error reading from Gemini WebSocket: {}", e);
                    break;
                }
                _ => { /* Ignore Ping/Pong */ }
            }
        }
    });

    Ok(GeminiClient {
        client_tx,
        server_tx,
    })
}

impl GeminiClient {
    /// Sends a message to the client's outgoing channel.
    async fn send_message(&self, message: ClientMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.client_tx
            .send(Message::Text(json))
            .await
            .context("Failed to send message to client channel")
    }

    /// Sends the initial session configuration.
    pub async fn send_config(&mut self, instructions: String) -> Result<()> {
        let setup = BidiGenerateContentSetup {
            model: "models/gemini-2.0-flash-exp".to_string(), // Model required by the API
            system_instruction: Some(Content {
                role: "user".to_string(),
                parts: vec![Part { text: instructions }],
            }),
            generation_config: Some(GenerationConfig {
                response_modalities: vec![ResponseModality::Audio, ResponseModality::Text],
            }),
        };
        self.send_message(ClientMessage::Setup(setup)).await
    }

    /// Sends a text-to-speech request.
    pub async fn send_tts(&mut self, text: String) -> Result<()> {
        let content = BidiGenerateContentClientContent {
            turns: vec![Content {
                role: "model".to_string(), // Using "model" role to make the AI speak our text
                parts: vec![Part { text }],
            }],
            turn_complete: true,
        };
        self.send_message(ClientMessage::ClientContent(content))
            .await
    }

    /// Sends a raw chunk of PCM audio data, base64 encoded.
    pub async fn send_audio_chunk(&mut self, base64_data: String) -> Result<()> {
        let input = BidiGenerateContentRealtimeInput {
            audio: Blob {
                mime_type: "audio/pcm;rate=16000".to_string(),
                data: base64_data,
            },
        };
        self.send_message(ClientMessage::RealtimeInput(input)).await
    }

    /// Subscribes to the stream of events from the server.
    pub fn server_events(&self) -> broadcast::Receiver<ServerMessage> {
        self.server_tx.subscribe()
    }
}
