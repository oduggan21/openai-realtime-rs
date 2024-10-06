use std::sync::{Arc, Mutex};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use crate::client::stats::Stats;
use crate::types;

mod consts;
mod config;
mod utils;
mod stats;

pub type ClientTx = tokio::sync::mpsc::Sender<types::ClientEvent>;
type ServerTx = tokio::sync::broadcast::Sender<types::ServerEvent>;
pub type ServerRx = tokio::sync::broadcast::Receiver<types::ServerEvent>;

pub struct Client {
    config: config::Config,
    c_tx: Option<ClientTx>,
    s_tx: Option<ServerTx>,


    stats: Arc<Mutex<Stats>>
}

impl Client {
    fn new(config: config::Config) -> Self {
        Self {
            config,
            c_tx: None,
            s_tx: None,
            stats: Arc::new(Mutex::new(Stats::new())),
        }
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.c_tx.is_some() {
            return Err("already connected".into());
        }

        let request = utils::build_request(&self.config)?;
        let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;

        let (mut write, read) = ws_stream.split();

        let (c_tx, mut c_rx) = tokio::sync::mpsc::channel(100);
        let (s_tx, _) = tokio::sync::broadcast::channel(100);

        self.c_tx = Some(c_tx.clone());
        self.s_tx = Some(s_tx.clone());

        tokio::spawn(async move {
            while let Some(event) = c_rx.recv().await {
                match serde_json::to_string(&event) {
                    Ok(text) => {
                        if let Err(e) = write.send(Message::Text(text)).await {
                            tracing::error!("failed to send message: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("failed to serialize event: {}", e);
                    }
                }
            }
        });

        let stats = self.stats.clone();
        tokio::spawn(async move {
            read.for_each(|message| async {
                match message {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<types::ServerEvent>(&text) {
                            Ok(event) => {
                                if let Err(e) = s_tx.send(event.clone()) {
                                    tracing::error!("failed to send event: {}", e);
                                }

                                if let types::ServerEvent::ResponseDone(response) = event {
                                    if let Some(usage) = response.response().usage() {
                                        let total_tokens = usage.total_tokens();
                                        let input_tokens = usage.input_tokens();
                                        let output_tokens = usage.output_tokens();
                                        
                                        if let Ok(mut stats_guard) = stats.lock() {
                                            stats_guard.update_usage(total_tokens, input_tokens, output_tokens);
                                        } else {
                                            tracing::error!("failed to update stats");
                                        }
                                        
                                        tracing::debug!("total_tokens: {}, input_tokens: {}, output_tokens: {}", total_tokens, input_tokens, output_tokens);
                                    }
                                }

                            }
                            Err(e) => {
                                let json = serde_json::from_str::<serde_json::Value>(&text);
                                json.map(|json| {
                                    tracing::error!("failed to deserialize event: {}, type=> {:?}", e, json);
                                }).unwrap_or_else(|_| {
                                    tracing::error!("failed to deserialize event: {}, text=> {:?}", e, text);
                                });
                                // tracing::error!("failed to deserialize event: {}, text=> {:?}", e, json);
                            }
                        }
                    }
                    Ok(Message::Binary(bin)) => {
                        tracing::warn!("unexpected binary message: {:?}", bin);
                    }
                    Err (e) => {
                        tracing::error!("failed to read message: {}", e);
                    }
                    _ => {}
                }
            }).await;
        });
        Ok(())
    }

    pub async fn server_events(&mut self) -> Result<ServerRx, Box<dyn std::error::Error>> {
        match self.s_tx {
            Some(ref tx) => Ok(tx.subscribe()),
            None => Err("not connected yet".into()),
        }
    }

    pub fn stats(&self) -> Result<Stats, Box<dyn std::error::Error>> {
        if let Ok(stats_guard) = self.stats.lock() {
            Ok(stats_guard.clone())
        } else {
            Err("failed to get stats".into())
        }
    }
    
    async fn send_client_event(&mut self, event: types::ClientEvent) -> Result<(), Box<dyn std::error::Error>> {
        match self.c_tx {
            Some(ref tx) => {
                tx.send(event).await?;
                Ok(())
            }
            None => Err("not connected yet".into()),
        }
    }
    
    pub async fn create_conversation_item(&mut self, item: types::Item) -> Result<(), Box<dyn std::error::Error>> {
        let event = types::ClientEvent::ConversationItemCreate(types::events::client::ConversationItemCreateEvent::new(item));
        self.send_client_event(event).await
    }
    
    pub async fn create_response(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event = types::ClientEvent::ResponseCreate(types::events::client::ResponseCreateEvent::new());
        self.send_client_event(event).await
    }
}

pub async fn connect_with_config(config: config::Config) -> Result<Client, Box<dyn std::error::Error>> {
    let mut client = Client::new(config);
    client.connect().await?;
    Ok(client)
}

pub async fn connect() -> Result<Client, Box<dyn std::error::Error>> {
    let config = config::Config::new();
    connect_with_config(config).await
}