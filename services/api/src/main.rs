use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Handles WebSocket upgrade requests.
///
/// This function is the entry point for WebSocket connections. It accepts the upgrade
/// request and passes the connection to `handle_socket` for processing.
async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    info!("WebSocket upgrade request received");
    ws.on_upgrade(handle_socket)
}

/// Manages an individual WebSocket connection.
///
/// Once a connection is established, this function will be responsible for the
/// communication logic (sending/receiving messages). For now, it just logs the
/// new connection and echoes any received messages.
async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket connection established");

    // In a real application, you would loop here to handle incoming messages
    // and interact with the feynman-core agent.
    // This loop is a placeholder to keep the connection alive and log messages.
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(msg) => {
                info!("Received message: {:?}", msg);
                // Echo the message back to the client.
                if socket.send(msg).await.is_err() {
                    // Client disconnected.
                    break;
                }
            }
            Err(e) => {
                // Client disconnected.
                info!("WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection closed");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber for logging.
    tracing_subscriber::fmt::init();

    // Configure a permissive CORS policy to allow connections from any origin.
    // This is necessary for a separate frontend to connect to the WebSocket API.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    // Create the Axum application router.
    let app = Router::new()
        // Define the WebSocket endpoint at `/ws`.
        .route("/ws", get(ws_handler))
        .layer(cors);

    // Define the address to bind the server to.
    let addr = "0.0.0.0:3000";
    info!("Starting WebSocket server, listening on {}", addr);

    // Create a TCP listener and bind it to the address.
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Run the server.
    axum::serve(listener, app).await?;

    Ok(())
}