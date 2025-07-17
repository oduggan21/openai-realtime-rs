# Development Guide

This guide provides everything you need to know to develop with, contribute to, or extend the `openai-realtime-rs` library.

## Quick Start

### Prerequisites
- Rust 2021 edition or later
- OpenAI API key with Realtime API access
- Audio devices for voice examples

### Environment Setup
```bash
# Clone the repository
git clone https://github.com/oduggan21/openai-realtime-rs.git
cd openai-realtime-rs

# Set up environment variables
echo "OPENAI_API_KEY=your_api_key_here" > .env

# Build the project
cargo build

# Run examples
cargo run --example hello      # Text-only example
cargo run --example devices    # List audio devices
cargo run --example voice      # Full voice interaction
```

## Project Structure

### Workspace Layout
This is a Cargo workspace with three crates:
- **Main crate** (`src/`): Core library implementation
- **Types crate** (`types/`): Data structures and type definitions
- **Utils crate** (`utils/`): Audio processing and device utilities

### Build Features
```toml
# Default features
cargo build

# Use rustls instead of native TLS
cargo build --no-default-features --features rustls

# Include utility functions
cargo build --features utils

# Minimal build (no TLS, no utils)
cargo build --no-default-features
```

## API Usage Patterns

### Basic Connection
```rust
use openai_realtime::{connect, types::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect with default configuration
    let mut client = connect().await?;
    
    // Get server event stream
    let mut events = client.server_events().await?;
    
    // Listen for events
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            println!("Received: {:?}", event);
        }
    });
    
    Ok(())
}
```

### Custom Configuration
```rust
use openai_realtime::client::config::Config;

let config = Config::builder()
    .with_api_key("your-api-key")
    .with_model("gpt-4o-realtime-preview-2024-10-01")
    .build();

let mut client = connect_with_config(1024, config).await?;
```

### Session Configuration
```rust
use openai_realtime::types::{Session, Voice, AudioFormat};

let session = Session::new()
    .with_modalities_enable_audio()
    .with_voice(Voice::Alloy)
    .with_input_audio_format(AudioFormat::Pcm16)
    .with_instructions("You are a helpful assistant")
    .build();

client.update_session(session).await?;
```

### Sending Messages
```rust
use openai_realtime::types::{MessageItem, MessageRole, Item};

// Create a user message
let message = MessageItem::builder()
    .with_role(MessageRole::User)
    .with_input_text("Hello, how are you?")
    .build();

// Add to conversation
client.create_conversation_item(Item::Message(message)).await?;

// Request response
client.create_response().await?;
```

### Audio Processing
```rust
use openai_realtime_utils::audio;

// Encode audio for sending
let audio_data: Vec<f32> = get_audio_samples();
let encoded = audio::encode_f32(&audio_data);
client.append_input_audio_buffer(encoded.into()).await?;

// Decode received audio
if let ServerEvent::ResponseAudioDelta(event) = server_event {
    let audio_data = audio::decode_f32(event.delta());
    play_audio(&audio_data);
}
```

## Audio Development

### Understanding Audio Flow

1. **Capture**: Microphone → CPAL → f32 samples
2. **Processing**: Resampling to 24kHz
3. **Encoding**: f32 → PCM16 → Base64
4. **Transmission**: WebSocket to OpenAI
5. **Reception**: Base64 from OpenAI
6. **Decoding**: Base64 → PCM16 → f32
7. **Playback**: Resampling → CPAL → Speakers

### Audio Device Setup
```rust
use openai_realtime_utils::device;

// List available devices
println!("Inputs:\n{}", device::get_available_inputs());
println!("Outputs:\n{}", device::get_available_outputs());

// Get specific devices
let input = device::get_or_default_input(Some("USB Microphone".to_string()))?;
let output = device::get_or_default_output(None)?; // Use default
```

### Real-time Audio Processing
```rust
use cpal::traits::{DeviceTrait, StreamTrait};
use openai_realtime_utils::audio;
use ringbuf::HeapRb;

// Create shared buffer
let buffer = audio::shared_buffer(48000); // 1 second at 48kHz
let (mut producer, mut consumer) = buffer.split();

// Audio input stream
let input_stream = input_device.build_input_stream(
    &config,
    move |data: &[f32], _: &cpal::InputCallbackInfo| {
        // Write to ring buffer
        producer.push_slice(data);
    },
    |err| eprintln!("Audio input error: {}", err),
    None,
)?;

// Processing loop
tokio::spawn(async move {
    let mut resampler = audio::create_resampler(48000.0, 24000.0, 1024)?;
    
    loop {
        // Read from ring buffer
        if consumer.occupied_len() >= 1024 {
            let mut samples = vec![0.0; 1024];
            consumer.pop_slice(&mut samples);
            
            // Resample to 24kHz
            let resampled = resampler.process(&[samples], None)?;
            
            // Encode and send
            let encoded = audio::encode_f32(&resampled[0]);
            client.append_input_audio_buffer(encoded.into()).await?;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
});
```

## Event Handling Patterns

### Basic Event Loop
```rust
let mut events = client.server_events().await?;

while let Ok(event) = events.recv().await {
    match event {
        ServerEvent::SessionCreated(e) => {
            println!("Session created: {:?}", e.session());
        }
        ServerEvent::ResponseTextDelta(e) => {
            print!("{}", e.delta());
            std::io::stdout().flush()?;
        }
        ServerEvent::ResponseAudioDelta(e) => {
            let audio = audio::decode_f32(e.delta());
            play_audio_chunk(&audio);
        }
        ServerEvent::Error(e) => {
            eprintln!("Error: {:?}", e);
        }
        _ => {} // Handle other events as needed
    }
}
```

### Async Event Processing
```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(100);
let mut events = client.server_events().await?;

// Event distributor
tokio::spawn(async move {
    while let Ok(event) = events.recv().await {
        if tx.send(event).await.is_err() {
            break;
        }
    }
});

// Specialized processors
let audio_tx = tx.clone();
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        match event {
            ServerEvent::ResponseAudioDelta(e) => {
                process_audio_chunk(e).await;
            }
            _ => {} // Ignore non-audio events
        }
    }
});
```

## Testing and Debugging

### Environment Variables
```bash
# Required for API access
export OPENAI_API_KEY="your-api-key"

# Optional: Enable debug logging
export RUST_LOG=debug

# Optional: Custom API endpoint
export OPENAI_BASE_URL="wss://api.openai.com/v1"
```

### Logging
The library uses the `tracing` crate for logging:

```rust
// In your application
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

### Common Issues

#### Audio Device Problems
```bash
# List available devices
cargo run --example devices

# Test with different sample rates
# Check device capabilities in the output
```

#### Connection Issues
- Verify API key is correct
- Check network connectivity
- Ensure Realtime API access is enabled

#### Audio Quality Issues
- Check sample rate conversion settings
- Verify buffer sizes are appropriate
- Monitor for dropouts or artifacts

## Performance Optimization

### Buffer Sizing
```rust
// Larger buffers = more latency, fewer dropouts
const CHUNK_SIZE: usize = 2048;  // ~85ms at 24kHz
const BUFFER_SIZE: usize = 24000; // 1 second

// Smaller buffers = less latency, more CPU usage
const CHUNK_SIZE: usize = 480;   // ~20ms at 24kHz
const BUFFER_SIZE: usize = 4800;  // 200ms
```

### Connection Tuning
```rust
// Larger capacity for high-throughput scenarios
let client = connect_with_config(4096, config).await?;

// Monitor statistics
println!("Stats: {:?}", client.stats()?);
```

## Extension Examples

### Custom Audio Processing
```rust
// Implement your own audio pipeline
struct CustomAudioProcessor {
    // Your custom fields
}

impl CustomAudioProcessor {
    fn process_input(&self, audio: &[f32]) -> Vec<f32> {
        // Apply noise reduction, filtering, etc.
        audio.to_vec()
    }
    
    fn process_output(&self, audio: &[f32]) -> Vec<f32> {
        // Apply EQ, volume control, etc.
        audio.to_vec()
    }
}
```

### Function Calling
```rust
use openai_realtime::types::tools::*;

// Define a function
let weather_tool = Tool::builder()
    .with_name("get_weather")
    .with_description("Get current weather")
    .with_parameter("location", ParameterType::String, "City name")
    .build();

// Add to session
let session = Session::new()
    .with_tools(vec![weather_tool])
    .with_tool_choice(ToolChoice::Auto)
    .build();

// Handle function calls
match event {
    ServerEvent::ResponseFunctionCallArgumentsDone(e) => {
        let result = execute_function(e.name(), e.arguments()).await?;
        // Send result back to conversation
    }
    _ => {}
}
```

### State Management
```rust
// Track conversation state
#[derive(Debug)]
struct ConversationState {
    items: Vec<Item>,
    current_response: Option<String>,
    audio_buffer: Vec<f32>,
}

impl ConversationState {
    fn handle_event(&mut self, event: ServerEvent) {
        match event {
            ServerEvent::ConversationItemCreated(e) => {
                self.items.push(e.item().clone());
            }
            ServerEvent::ResponseTextDelta(e) => {
                self.current_response
                    .get_or_insert_with(String::new)
                    .push_str(e.delta());
            }
            _ => {}
        }
    }
}
```

## Contributing

### Code Style
- Use `rustfmt` for formatting
- Follow Rust naming conventions
- Add documentation for public APIs
- Include examples in documentation

### Pull Request Process
1. Fork the repository
2. Create a feature branch
3. Make minimal, focused changes
4. Add tests if applicable
5. Update documentation
6. Submit pull request

### Testing Guidelines
Currently, the project lacks comprehensive tests. Contributions adding tests are welcome:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection() {
        // Test basic connection functionality
    }
    
    #[test]
    fn test_audio_encoding() {
        // Test audio encoding/decoding
    }
}
```

This guide should give you everything needed to effectively develop with the `openai-realtime-rs` library. For more specific questions, refer to the examples in the `examples/` directory or examine the source code directly.