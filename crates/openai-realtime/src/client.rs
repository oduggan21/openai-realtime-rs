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

//contains the capacity which sets the size of the channels, our client and server transmitters, config
//and stats which is gaurded
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
        //ensure that we haven't already connected
        if self.c_tx.is_some() {
            // Use anyhow's error type
            return Err(anyhow::anyhow!("already connected"));
        }

        //create a request using the build request function
        let request = utils::build_request(&self.config)?;

        //get a tcpStream object
        let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;

        //split the websocket into read and write
        let (mut write, mut read) = ws_stream.split();

        //create the channels to hold events to send and reveive
        let (c_tx, mut c_rx) = tokio::sync::mpsc::channel(self.capacity);
        //create the server transmitter that will broadcast out to client recievers
        let (s_tx, _) = tokio::sync::broadcast::channel(self.capacity);

        //clone the server and client transmitters into the struct
        self.c_tx = Some(c_tx.clone());
        self.s_tx = Some(s_tx.clone());

        //this function creates a listening task that will listen for events on the client recieving channel
        tokio::spawn(async move {
            while let Some(event) = c_rx.recv().await {
                match serde_json::to_string(&event) {
                    //take the json event and attempt to send it using our websocket
                    Ok(text) => {
                        //if we hav an error sending message output it
                        if let Err(e) = write.send(Message::Text(text)).await {
                            tracing::error!("failed to send message: {}", e);
                        }
                    }
                    //if we get an error converting the json say we failed to seriailze it
                    Err(e) => {
                        tracing::error!("failed to serialize event: {}", e);
                    }
                }
            }
        });

        let stats = self.stats.clone();
        //spawn event to listen to server event and transmit those out, we first need to ensure we can get a message
        //next we need to verify the message type(text, binary, close), then we need to get the event type
        //we use the read websocket channel to receive messages
        //we use server transmitter to broadcast events
        //
        tokio::spawn(async move {
            //get the message from the websocket, it will come in json format
            while let Some(message) = read.next().await {
                //message will be equal to the message received or
                let message = match message {
                    Err(e) => {
                        tracing::error!("failed to read message: {}", e);
                        break;
                    }
                    Ok(message) => message,
                };
                //at this point we have the message
                //match the message variant to text, binary, or close
                match message {
                    Message::Text(text) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            //get the event type as a string
                            let event_type = json.get("type").map(|v| v.as_str()).flatten();
                            //get the event id as a string
                            let event_id = json.get("event_id").map(|v| v.as_str()).flatten();
                            //track the messages
                            tracing::debug!(
                                "received message: {}, id={}",
                                event_type.unwrap_or("unknown"),
                                event_id.unwrap_or("unknown")
                            );
                        }
                        //match the servent event enum variant or get an error
                        match serde_json::from_str::<types::ServerEvent>(&text) {
                            Ok(event) => {
                                //send the server event across the transmitting server channel
                                if let Err(e) = s_tx.send(event.clone()) {
                                    tracing::error!("failed to send event: {}", e);
                                }

                                //if the server is done responding record its usage stats
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
                            //error message to say that we couldn't properly detect the server event
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
                                // tracing::error!("failed to deserialize event: {}, text=> {:?}", e, json);
                            }
                        }
                    }
                    //we get a binary message and not json
                    Message::Binary(bin) => {
                        tracing::warn!("unexpected binary message: {:?}", bin);
                    }
                    //we got that the websocket is closed
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

    //get a server receiver that we can use to receive server events

    pub async fn server_events(&mut self) -> Result<ServerRx> {
        match self.s_tx {
            Some(ref tx) => Ok(tx.subscribe()),
            None => Err(anyhow::anyhow!("not connected yet")),
        }
    }

    //return a status objct that we can use to inspect that stats

    pub fn stats(&self) -> Result<Stats> {
        if let Ok(stats_guard) = self.stats.lock() {
            Ok(stats_guard.clone())
        } else {
            Err(anyhow::anyhow!("failed to get stats"))
        }
    }

    ///send the client event

    async fn send_client_event(&mut self, event: types::ClientEvent) -> Result<()> {
        match self.c_tx {
            Some(ref tx) => {
                tx.send(event).await?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("not connected yet")),
        }
    }

    //function to send update session event
    pub async fn update_session(&mut self, config: Session) -> Result<()> {
        let event = types::ClientEvent::SessionUpdate(
            types::events::client::SessionUpdateEvent::new(config),
        );
        self.send_client_event(event).await
    }

    //funciton to send input audio buffer event
    pub async fn append_input_audio_buffer(
        &mut self,
        audio: Base64EncodedAudioBytes,
    ) -> Result<()> {
        let event = types::ClientEvent::InputAudioBufferAppend(
            types::events::client::InputAudioBufferAppendEvent::new(audio),
        );
        self.send_client_event(event).await
    }

    //function to send conversation item event
    pub async fn create_conversation_item(&mut self, item: types::Item) -> Result<()> {
        let event = types::ClientEvent::ConversationItemCreate(
            types::events::client::ConversationItemCreateEvent::new(item),
        );
        self.send_client_event(event).await
    }

    //function to send create response event
    pub async fn create_response(&mut self) -> Result<()> {
        let event =
            types::ClientEvent::ResponseCreate(types::events::client::ResponseCreateEvent::new());
        self.send_client_event(event).await
    }

    //function to send create response with specfic config event
    pub async fn create_response_with_config(&mut self, config: Session) -> Result<()> {
        let event = types::ClientEvent::ResponseCreate(
            types::events::client::ResponseCreateEvent::new().with_update_session(config),
        );
        self.send_client_event(event).await
    }
}

//public function to create client with specific config and connect to open ai
pub async fn connect_with_config(capacity: usize, config: config::Config) -> Result<Client> {
    let mut client = Client::new(capacity, config);
    client.connect().await?;
    Ok(client)
}

//public function
pub async fn connect() -> Result<Client> {
    //create the default config object
    let config = config::Config::new();
    //call connect with config
    connect_with_config(1024, config).await
}
