use crate::types::{ConfigRequest, ServerEvent, SessionConfig, TtsPayload, TtsRequest};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};

type WsWriter =
    futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsReader = futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// A client for the simplified Gemini Real-time WebSocket API.
pub struct GeminiClient {
    write: WsWriter,
    read: WsReader,
}

/// Establishes a connection to the Gemini real-time service.
pub async fn connect(api_key: &str) -> Result<GeminiClient> {
    let url = format!("wss://gemini.api.google.com/v1/stream?key={}", api_key);
    let (ws_stream, _) = connect_async(url)
        .await
        .context("Failed to connect to Gemini WebSocket")?;

    tracing::info!("Successfully connected to Gemini WebSocket.");
    let (write, read) = ws_stream.split();
    Ok(GeminiClient { write, read })
}

impl GeminiClient {
    /// Sends the initial session configuration.
    pub async fn send_config(&mut self, instructions: String) -> Result<()> {
        let req = ConfigRequest {
            config: SessionConfig { instructions },
        };
        let json = serde_json::to_string(&req)?;
        self.write
            .send(Message::Text(json))
            .await
            .context("Failed to send config message")
    }

    /// Sends a text-to-speech request.
    pub async fn send_tts(&mut self, text: String) -> Result<()> {
        let req = TtsRequest {
            tts: TtsPayload { text },
        };
        let json = serde_json::to_string(&req)?;
        self.write
            .send(Message::Text(json))
            .await
            .context("Failed to send TTS message")
    }

    /// Sends a raw chunk of PCM audio data.
    pub async fn send_audio_chunk(&mut self, pcm_data: Vec<u8>) -> Result<()> {
        self.write
            .send(Message::Binary(pcm_data))
            .await
            .context("Failed to send audio chunk")
    }

    /// Reads the next event from the server.
    pub async fn next_event(&mut self) -> Result<Option<ServerEvent>> {
        while let Some(msg) = self.read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let event: ServerEvent = serde_json::from_str(&text)
                        .context("Failed to deserialize server event")?;
                    return Ok(Some(event));
                }
                Ok(Message::Binary(_)) => {
                    tracing::warn!("Received unexpected binary message from Gemini server.");
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Gemini WebSocket connection closed.");
                    return Ok(None);
                }
                Err(e) => {
                    tracing::error!("Error reading from Gemini WebSocket: {}", e);
                    return Err(e.into());
                }
                _ => { /* Ignore Ping/Pong */ }
            }
        }
        Ok(None)
    }
}
