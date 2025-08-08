use anyhow::{Context, Result};
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use feynman_core::{
    agent::{FeynmanAgent, FeynmanService},
    reviewer::{OpenAIReviewer, Reviewer}, // Correctly import the Reviewer trait
    topic::{SubTopic, SubTopicList},
};
use rmcp::{ServiceExt, model::CallToolRequestParam, object};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

// --- JSON Message Protocol ---

/// Messages sent from the browser client to the server.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "user_message")]
    UserMessage { text: String },
}

/// Messages sent from the server to the browser client.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "agent_response")]
    AgentResponse { text: String },
    #[serde(rename = "error")]
    Error { message: String },
}

// --- WebSocket Handlers ---

/// Handles WebSocket upgrade requests.
/// This function is the entry point for WebSocket connections.
async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    info!("WebSocket upgrade request received");
    ws.on_upgrade(handle_socket)
}

/// Manages an individual WebSocket connection and its dedicated agent session.
async fn handle_socket(socket: WebSocket) {
    info!("New WebSocket connection established. Initializing agent session...");

    // Spawn a dedicated task for this client's agent session.
    // The `tokio::spawn` moves the socket and all agent logic into a new task,
    // allowing the main server to continue accepting new connections.
    tokio::spawn(async move {
        // The `run_agent_session` function contains the core logic for a single session.
        // We wrap it to handle and log any potential errors gracefully without crashing the server.
        if let Err(e) = run_agent_session(socket).await {
            error!("Agent session failed: {:?}", e);
        }
        info!("Agent session task finished.");
    });
}

/// The core logic for a single user session.
///
/// This function performs all the setup for a new agent, bridges the WebSocket
/// to the agent's MCP transport, and runs the communication loop.
async fn run_agent_session(mut socket: WebSocket) -> Result<()> {
    // --- 1. Agent Initialization ---
    dotenvy::dotenv().ok();
    let prompts = feynman_service::prompt_loader::load_prompts(Path::new("prompts"))
        .context("Failed to load LLM prompts from './prompts' directory")?;
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY must be set")?;
    let model = std::env::var("CHAT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

    let reviewer: Arc<dyn Reviewer> = Arc::new(OpenAIReviewer::new(api_key, model, prompts));

    // For this web service, we'll hardcode the topic.
    let main_topic = "The Feynman Technique".to_string();
    let subtopic_names = reviewer.generate_subtopics(&main_topic).await?;
    let subtopics: Vec<SubTopic> = subtopic_names.into_iter().map(SubTopic::new).collect();
    let subtopic_list = SubTopicList::new(subtopics);

    let agent_state = Arc::new(tokio::sync::Mutex::new(FeynmanAgent::new(
        main_topic,
        subtopic_list,
    )));
    let feynman_service = FeynmanService::new(agent_state, reviewer.clone());

    // --- 2. MCP Transport Setup ---
    // Create an in-memory, asynchronous pipe for the agent to communicate through.
    // This creates a pair of connected streams that implement AsyncRead + AsyncWrite.
    let (client_io, server_io) = tokio::io::duplex(4096);

    // Spawn the agent's main service loop in a separate task.
    let agent_handle = tokio::spawn(async move {
        info!("Starting FeynmanService for new client.");
        if let Err(e) = feynman_service.serve(server_io).await {
            error!("Agent service loop exited with error: {:?}", e);
        }
        info!("FeynmanService for client has shut down.");
    });

    // Create the client-side handle to communicate with the agent.
    let mcp_client = ().serve(client_io).await?;

    info!("Agent session initialized successfully. Ready for messages.");

    // --- 3. Bidirectional Communication Loop ---
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
                        // Call the agent's `send_message` tool via the MCP client.
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
                                    // Success path
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
                                    // Error path
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

                        // Send the agent's response back to the browser.
                        let server_msg = ServerMessage::AgentResponse {
                            text: response_text,
                        };
                        let payload = serde_json::to_string(&server_msg)?;
                        socket.send(Message::Text(payload.into())).await?;
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
            _ => { // Ignore other message types like Binary, Ping, Pong
            }
        }
    }

    // --- 4. Graceful Shutdown ---
    // The loop has exited, meaning the client disconnected.
    // Aborting the agent task ensures its resources are cleaned up.
    agent_handle.abort();
    info!("WebSocket connection closed and agent session terminated.");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .init();

    // Configure a permissive CORS policy.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create the Axum application router.
    let app = Router::new().route("/ws", get(ws_handler)).layer(cors);

    // Bind the server to an address and start it.
    let addr = "0.0.0.0:3000";
    info!("Starting WebSocket server, listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}