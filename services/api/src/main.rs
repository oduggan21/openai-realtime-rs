mod config;

use crate::config::{Config, Provider};
use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use feynman_core::{
    agent::{FeynmanAgent, FeynmanService},
    gemini_reviewer::GeminiReviewer,
    reviewer::{OpenAIReviewer, Reviewer},
    topic::{SubTopic, SubTopicList},
};
use rmcp::{ServiceExt, model::CallToolRequestParam, object};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

// --- Application State ---

/// Holds shared application state, created once at startup.
pub struct AppState {
    reviewer: Arc<dyn Reviewer>,
}

// --- JSON Message Protocol ---

/// Messages sent from the browser client to the server.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "init")]
    Init { topic: String },
    #[serde(rename = "user_message")]
    UserMessage { text: String },
}

/// Messages sent from the server to the browser client.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "initialized")]
    Initialized {
        main_topic: String,
        subtopics: Vec<String>,
    },
    #[serde(rename = "agent_response")]
    AgentResponse { text: String },
    #[serde(rename = "error")]
    Error { message: String },
}

// --- WebSocket Handlers ---

/// Handles WebSocket upgrade requests.
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    info!("WebSocket upgrade request received");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Manages an individual WebSocket connection and its dedicated agent session.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    info!("New WebSocket connection established. Initializing agent session...");
    tokio::spawn(async move {
        if let Err(e) = run_agent_session(socket, state).await {
            // Using `?` formatting for detailed error logging.
            error!("Agent session failed: {:?}", e);
        }
        info!("Agent session task finished.");
    });
}

/// The core logic for a single user session.
async fn run_agent_session(mut socket: WebSocket, state: Arc<AppState>) -> Result<()> {
    // --- Phase 1: Initialization ---
    // The first message from the client MUST be an 'init' message with the topic.
    let topic = match socket.recv().await {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Init { topic }) => {
                info!("Initializing session for topic: '{}'", topic);
                topic
            }
            _ => return Err(anyhow!("First message was not a valid 'init' message.")),
        },
        _ => return Err(anyhow!("Client disconnected before sending init message.")),
    };

    // --- 2. Agent Initialization ---
    let reviewer = state.reviewer.clone();
    let subtopic_names = reviewer.generate_subtopics(&topic).await?;
    // Use `into_iter` to pass owned `String`s to `SubTopic::new`.
    let subtopics: Vec<SubTopic> = subtopic_names
        .clone()
        .into_iter()
        .map(SubTopic::new)
        .collect();
    let subtopic_list = SubTopicList::new(subtopics);

    let agent_state = Arc::new(tokio::sync::Mutex::new(FeynmanAgent::new(
        topic.clone(),
        subtopic_list,
    )));
    let feynman_service = FeynmanService::new(agent_state, reviewer);

    // --- 3. MCP Transport Setup ---
    let (client_io, server_io) = tokio::io::duplex(4096);
    let agent_handle = tokio::spawn(async move {
        info!("Starting FeynmanService for new client.");
        // The `serve` method returns a `RunningService`. We must hold onto it
        // and call `.waiting()` to keep the service alive.
        match feynman_service.serve(server_io).await {
            Ok(running_service) => {
                info!("FeynmanService is running. Waiting for completion...");
                if let Err(e) = running_service.waiting().await {
                    error!("Agent service waiting loop exited with error: {:?}", e);
                }
            }
            Err(e) => {
                error!("Agent service failed to start: {:?}", e);
            }
        }
        info!("FeynmanService for client has shut down.");
    });
    let mcp_client = ().serve(client_io).await?;
    info!("Agent session initialized successfully. Ready for messages.");

    // --- 4. Signal to client that we are ready ---
    let ready_msg = ServerMessage::Initialized {
        main_topic: topic,
        subtopics: subtopic_names,
    };
    let payload = serde_json::to_string(&ready_msg)?;
    socket.send(Message::Text(payload.into())).await?;

    // --- 5. Bidirectional Communication Loop ---
    while let Some(msg_result) = socket.recv().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                let client_msg: ClientMessage = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        warn!("Failed to deserialize client message: {}. Raw: {}", e, text);
                        let err_msg = ServerMessage::Error {
                            message: format!("Invalid message format: {}", e),
                        };
                        let payload = serde_json::to_string(&err_msg)?;
                        socket.send(Message::Text(payload.into())).await?;
                        continue;
                    }
                };

                info!("Received from client: {:?}", client_msg);

                match client_msg {
                    ClientMessage::UserMessage { text } => {
                        let call_result = mcp_client
                            .peer()
                            .call_tool(CallToolRequestParam {
                                name: "send_message".into(),
                                arguments: Some(object!({ "text": text })),
                            })
                            .await;

                        let response_text = match call_result {
                            Ok(tool_result) => {
                                if !tool_result.is_error.unwrap_or(false) {
                                    tool_result
                                        .content
                                        .and_then(|mut c| c.pop())
                                        .and_then(|c| c.as_text().map(|t| t.text.clone()))
                                        .and_then(|json_str| {
                                            serde_json::from_str::<String>(&json_str).ok()
                                        })
                                        .unwrap_or_else(|| {
                                            "Agent returned unexpected data format.".to_string()
                                        })
                                } else {
                                    let error_message = tool_result
                                        .content
                                        .and_then(|mut c| c.pop())
                                        .and_then(|c| c.as_text().map(|t| t.text.clone()))
                                        .unwrap_or_else(|| "Unknown tool error".to_string());
                                    format!("Agent error: {}", error_message)
                                }
                            }
                            Err(rpc_err) => {
                                error!("MCP RPC error: {:?}", rpc_err);
                                "A communication error occurred with the agent.".to_string()
                            }
                        };

                        let server_msg = ServerMessage::AgentResponse {
                            text: response_text,
                        };
                        let payload = serde_json::to_string(&server_msg)?;
                        socket.send(Message::Text(payload.into())).await?;
                    }
                    ClientMessage::Init { .. } => {
                        warn!("Client sent 'init' message after session was already initialized.");
                        // Optionally send an error message back to the client.
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client sent close frame. Shutting down session.");
                break;
            }
            Err(e) => {
                error!("WebSocket receive error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // --- 6. Graceful Shutdown ---
    agent_handle.abort();
    info!("WebSocket connection closed and agent session terminated.");
    Ok(())
}

/// Listens for the `Ctrl+C` signal to gracefully shut down the server.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    info!("Received shutdown signal. Shutting down gracefully...");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- 1. Load Configuration ---
    let config = Config::from_env().context("Failed to load configuration")?;

    // --- 2. Initialize Logging ---
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .init();

    info!("Configuration loaded. Initializing application state...");

    // --- 3. Initialize Shared State ---
    let prompts = feynman_service::prompt_loader::load_prompts(Path::new("prompts"))
        .context("Failed to load LLM prompts from './prompts' directory")?;

    let reviewer: Arc<dyn Reviewer> = match config.provider {
        Provider::OpenAI => {
            info!("Using OpenAI provider for Reviewer service.");
            Arc::new(OpenAIReviewer::new(
                config.openai_api_key.unwrap(), // Safe due to check in Config
                config.chat_model,
                prompts,
            ))
        }
        Provider::Gemini => {
            info!("Using Gemini provider for Reviewer service.");
            // The GeminiReviewer is a mock/placeholder for now.
            Arc::new(GeminiReviewer)
        }
    };

    let app_state = Arc::new(AppState { reviewer });

    // --- 4. Configure Server ---
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(app_state);

    // --- 5. Start Server with Graceful Shutdown ---
    info!(
        "Starting WebSocket server, listening on {}",
        config.bind_address
    );
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server has shut down.");
    Ok(())
}
