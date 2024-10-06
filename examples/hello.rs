use openai_realtime_types::Item;

#[tokio::main]
async fn main() {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let mut client = openai_realtime::connect().await.expect("failed to connect");

    let mut server_events = client.server_events().await.expect("failed to get server events");
    
    println!("Connected to OpenAI Realtime API");
    tokio::spawn(
        async move {
            while let Ok(e) = server_events.recv().await {
                println!("{:?}", e);
            }
        }
    );

    let message = openai_realtime::types::MessageItem::builder()
        .with_input_text("Hello!")
        .build();

    client.create_conversation_item(Item::Message(message)).await.expect("failed to send message");
    client.create_response().await.expect("failed to send message");

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}
