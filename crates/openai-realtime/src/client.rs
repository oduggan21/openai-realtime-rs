use crate::client::stats::Stats;
use crate::types;
use futures_util::{SinkExt, StreamExt};
use openai_realtime_types::audio::Base64EncodedAudioBytes;
use openai_realtime_types::session::Session;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::tungstenite::Message;
// Add this use statement
use anyhow::Result;

mod config;
mod consts;
mod stats;
mod utils;

pub type ClientTx = tokio::sync::mpsc::Sender<types::ClientEvent>;
type ServerTx = tokio::sync::broadcast::Sender<types::ServerEvent>;
pub type ServerRx = tokio::sync::broadcast::Receiver<types::ServerEvent>;

// Contains the capacity for channels, client/server transmitters, configuration,
// and stats guarded by a Mutex.
pub struct Client {
    capacity: usize,
    config: config::Config,
    c_tx: Option<ClientTx>,
    s_tx: Option<ServerTx>,
    stats: Arc<Mutex<Stats>>,
}

impl Client {
    fn new(capacity: usize, config: config::Config) -> Self {
        Self {
            capacity,
            config,
            c_tx: None,
            s_tx: None,
            stats: Arc::new(Mutex::new(Stats::new())),
        }
    }

    async fn connect(&mut self) -> Result<()> {
        // Ensure that we haven't already connected.
        if self.c_tx.is_some() {
            // Use anyhow's error type for clear error messages.
            return Err(anyhow::anyhow!("already connected"));
        }

        // Create a request using the build_request function.
        let request = utils::build_request(&self.config)?;

        // Get a WebSocket stream object.
        let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;

        // Split the WebSocket into read and write halves.
        let (mut write, mut read) = ws_stream.split();

        // Create the channels to hold events to send and receive.
        let (c_tx, mut c_rx) = tokio::sync::mpsc::channel(self.capacity);
        // Create the server transmitter that will broadcast out to client receivers.
        let (s_tx, _) = tokio::sync::broadcast::channel(self.capacity);

        // Store the server and client transmitters in the struct.
        self.c_tx = Some(c_tx.clone());
        self.s_tx = Some(s_tx.clone());

        // This task listens for events on the client receiving channel.
        tokio::spawn(async move {
            while let Some(event) = c_rx.recv().await {
                match serde_json::to_string(&event) {
                    // Take the JSON event and attempt to send it using our WebSocket.
                    Ok(text) => {
                        // If we have an error sending the message, output it.
                        if let Err(e) = write.send(Message::Text(text)).await {
                            tracing::error!("failed to send message: {}", e);
                        }
                    }
                    // If we get an error converting to JSON, log the serialization failure.
                    Err(e) => {
                        tracing::error!("failed to serialize event: {}", e);
                    }
                }
            }
        });

        let stats = self.stats.clone();
        // Spawn a task to listen for server events and transmit them.
        // We first need to ensure we can get a message, then verify its type (text, binary, close),
        // and finally determine the event type.
        // We use the `read` half of the WebSocket to receive messages and the `s_tx` to broadcast events.
        tokio::spawn(async move {
            // Get the message from the WebSocket, which will be in JSON format.
            while let Some(message) = read.next().await {
                let message = match message {
                    Err(e) => {
                        tracing::error!("failed to read message: {}", e);
                        break;
                    }
                    Ok(message) => message,
                };
                // At this point, we have the message.
                // Match the message variant to handle text, binary, or close messages.
                match message {
                    Message::Text(text) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            // Get the event type and ID as strings for logging.
                            let event_type = json.get("type").and_then(|v| v.as_str());
                            let event_id = json.get("event_id").and_then(|v| v.as_str());
                            // Track the received messages.
                            tracing::debug!(
                                "received message: {}, id={}",
                                event_type.unwrap_or("unknown"),
                                event_id.unwrap_or("unknown")
                            );
                        }
                        // Match the server event enum variant or handle the error.
                        match serde_json::from_str::<types::ServerEvent>(&text) {
                            Ok(event) => {
                                // Send the server event across the transmitting server channel.
                                if let Err(e) = s_tx.send(event.clone()) {
                                    tracing::error!("failed to send event: {}", e);
                                }

                                // If the server is done responding, record its usage stats.
                                if let types::ServerEvent::ResponseDone(response) = event {
                                    if let Some(usage) = response.response().usage() {
                                        let total_tokens = usage.total_tokens();
                                        let input_tokens = usage.input_tokens();
                                        let output_tokens = usage.output_tokens();

                                        if let Ok(mut stats_guard) = stats.lock() {
                                            stats_guard.update_usage(
                                                total_tokens,
                                                input_tokens,
                                                output_tokens,
                                            );
                                        } else {
                                            tracing::error!("failed to update stats");
                                        }

                                        tracing::debug!(
                                            "total_tokens: {}, input_tokens: {}, output_tokens: {}",
                                            total_tokens,
                                            input_tokens,
                                            output_tokens
                                        );
                                    }
                                }
                            }
                            // Log an error if we couldn't properly deserialize the server event.
                            Err(e) => {
                                let json = serde_json::from_str::<serde_json::Value>(&text);
                                json.map(|json| {
                                    tracing::error!(
                                        "failed to deserialize event: {}, type=> {:?}",
                                        e,
                                        json
                                    );
                                })
                                .unwrap_or_else(|_| {
                                    tracing::error!(
                                        "failed to deserialize event: {}, text=> {:?}",
                                        e,
                                        text
                                    );
                                });
                            }
                        }
                    }
                    // We received a binary message, not JSON.
                    Message::Binary(bin) => {
                        tracing::warn!("unexpected binary message: {:?}", bin);
                    }
                    // The WebSocket connection was closed.
                    Message::Close(reason) => {
                        tracing::info!("connection closed: {:?}", reason);
                        let close_event = types::ServerEvent::Close {
                            reason: reason.map(|v| format!("{:?}", v)),
                        };
                        if let Err(e) = s_tx.send(close_event) {
                            tracing::error!("failed to send close event: {}", e);
                        }
                        break;
                    }
                    _ => {}
                }
            }
            drop(c_tx);
            drop(s_tx);
        });
        Ok(())
    }

    // Get a server receiver that we can use to receive server events.
    pub async fn server_events(&mut self) -> Result<ServerRx> {
        match self.s_tx {
            Some(ref tx) => Ok(tx.subscribe()),
            None => Err(anyhow::anyhow!("not connected yet")),
        }
    }

    // Return a stats object that we can use to inspect the stats.
    pub fn stats(&self) -> Result<Stats> {
        if let Ok(stats_guard) = self.stats.lock() {
            Ok(stats_guard.clone())
        } else {
            Err(anyhow::anyhow!("failed to get stats"))
        }
    }

    /// Send a client event.
    async fn send_client_event(&mut self, event: types::ClientEvent) -> Result<()> {
        match self.c_tx {
            Some(ref tx) => {
                tx.send(event).await?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("not connected yet")),
        }
    }

    // Function to send an update session event.
    pub async fn update_session(&mut self, config: Session) -> Result<()> {
        let event = types::ClientEvent::SessionUpdate(
            types::events::client::SessionUpdateEvent::new(config),
        );
        self.send_client_event(event).await
    }

    // Function to send an input audio buffer event.
    pub async fn append_input_audio_buffer(
        &mut self,
        audio: Base64EncodedAudioBytes,
    ) -> Result<()> {
        let event = types::ClientEvent::InputAudioBufferAppend(
            types::events::client::InputAudioBufferAppendEvent::new(audio),
        );
        self.send_client_event(event).await
    }

    // Function to send a conversation item event.
    pub async fn create_conversation_item(&mut self, item: types::Item) -> Result<()> {
        let event = types::ClientEvent::ConversationItemCreate(
            types::events::client::ConversationItemCreateEvent::new(item),
        );
        self.send_client_event(event).await
    }

    // Function to send a create response event.
    pub async fn create_response(&mut self) -> Result<()> {
        let event =
            types::ClientEvent::ResponseCreate(types::events::client::ResponseCreateEvent::new());
        self.send_client_event(event).await
    }

    // Function to send a create response event with a specific config.
    pub async fn create_response_with_config(&mut self, config: Session) -> Result<()> {
        let event = types::ClientEvent::ResponseCreate(
            types::events::client::ResponseCreateEvent::new().with_update_session(config),
        );
        self.send_client_event(event).await
    }
}

// Public function to create a client with specific config and connect to OpenAI.
pub async fn connect_with_config(capacity: usize, config: config::Config) -> Result<Client> {
    let mut client = Client::new(capacity, config);
    client.connect().await?;
    Ok(client)
}

// Public function to connect with default settings.
pub async fn connect() -> Result<Client> {
    // Create the default config object.
    let config = config::Config::new();
    // Call connect_with_config using the default config.
    connect_with_config(1024, config).await
}
