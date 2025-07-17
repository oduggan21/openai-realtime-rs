# Code Architecture and Component Interaction Guide

This document provides a deep dive into how the components of `openai-realtime-rs` work together, including data flow, interaction patterns, and implementation details.

## Core Architecture

### High-Level Component Diagram

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Application   │    │   Audio Utils   │    │   Device Mgmt   │
│   (examples/)   │    │   (utils/audio) │    │  (utils/device) │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     Main Library        │
                    │      (src/)             │
                    │ ┌─────────────────────┐ │
                    │ │      Client         │ │
                    │ │   (client.rs)       │ │
                    │ └─────────────────────┘ │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │       Types             │
                    │     (types/)            │
                    │ ┌─────────┬─────────┐   │
                    │ │ Events  │ Session │   │
                    │ │ Content │  Audio  │   │
                    │ └─────────┴─────────┘   │
                    └─────────────────────────┘
```

## Detailed Component Analysis

### 1. Client Implementation (`src/client.rs`)

The `Client` struct is the core of the library, managing the WebSocket connection and event handling.

#### Key Components:
```rust
pub struct Client {
    capacity: usize,           // Channel buffer size
    config: config::Config,    // Connection configuration
    c_tx: Option<ClientTx>,    // Channel for sending to server
    s_tx: Option<ServerTx>,    // Broadcast channel for server events
    stats: Arc<Mutex<Stats>>   // Thread-safe usage statistics
}
```

#### Connection Flow:
1. **WebSocket Establishment**: Creates authenticated WebSocket connection
2. **Channel Setup**: Establishes bidirectional communication channels
3. **Spawn Tasks**: Creates async tasks for read/write operations
4. **Event Processing**: Continuous message handling loop

#### Critical Methods:
- `connect()`: Establishes WebSocket connection and spawns handlers
- `send_client_event()`: Sends events to OpenAI API
- `server_events()`: Returns receiver for server events
- Convenience methods: `update_session()`, `append_input_audio_buffer()`, etc.

### 2. Configuration System (`src/client/config.rs`)

#### Configuration Structure:
```rust
pub struct Config {
    base_url: String,        // API endpoint URL
    api_key: SecretString,   // Securely stored API key
    model: String,           // Model identifier
}
```

#### Builder Pattern Implementation:
```rust
Config::builder()
    .with_base_url("wss://api.openai.com/v1")
    .with_api_key(api_key)
    .with_model("gpt-4o-realtime-preview-2024-10-01")
    .build()
```

### 3. Event System (`types/src/events/`)

The event system is the backbone of communication with the OpenAI API.

#### Event Flow:
```
Application → ClientEvent → JSON → WebSocket → OpenAI API
                                                    ↓
Application ← ServerEvent ← JSON ← WebSocket ← OpenAI API
```

#### Key Event Types:

**Client Events** (Outbound):
- `SessionUpdate`: Modify session configuration
- `InputAudioBufferAppend`: Send audio data
- `ConversationItemCreate`: Add messages/items
- `ResponseCreate`: Request AI response

**Server Events** (Inbound):
- `SessionCreated/Updated`: Session status
- `ResponseAudioDelta`: Streaming audio response
- `ResponseTextDelta`: Streaming text response
- `ConversationItemCreated`: Acknowledgment of new items

### 4. Session Management (`types/src/session.rs`)

Sessions define the behavior and capabilities of the AI interaction.

#### Session Builder Pattern:
```rust
Session::new()
    .with_modalities_enable_audio()
    .with_voice(Voice::Alloy)
    .with_instructions("You are a helpful assistant")
    .with_input_audio_format(AudioFormat::Pcm16)
    .with_turn_detection_enable(TurnDetection::server_vad())
    .build()
```

#### Key Configuration Areas:
- **Modalities**: Text, audio, or both
- **Voice Settings**: AI voice selection
- **Audio Formats**: Input/output audio configuration
- **Turn Detection**: When to process audio input
- **Tools**: Function calling capabilities

### 5. Audio Processing Pipeline (`utils/src/audio.rs`)

The audio utilities handle the complex task of converting between different audio formats and sample rates.

#### Audio Flow:
```
Microphone → Device Driver → CPAL → f32 samples → Resampling → 
PCM16 → Base64 → API → Base64 → PCM16 → f32 samples → 
Resampling → CPAL → Device Driver → Speakers
```

#### Key Functions:
- `encode_f32()`: Convert f32 samples to base64 PCM16
- `decode_f32()`: Convert base64 PCM16 to f32 samples
- `create_resampler()`: Setup sample rate conversion
- `shared_buffer()`: Create ring buffers for real-time processing

#### Sample Rate Conversion:
OpenAI expects 24kHz PCM16, but most audio devices use different rates (44.1kHz, 48kHz). The resampling utilities handle this conversion transparently.

### 6. Device Management (`utils/src/device.rs`)

Provides cross-platform audio device access using CPAL.

#### Device Selection Flow:
1. **Host Selection**: Choose audio system (ALSA, CoreAudio, WASAPI, etc.)
2. **Device Enumeration**: List available devices with capabilities
3. **Configuration**: Select appropriate sample rate and buffer size
4. **Stream Creation**: Setup audio streams for recording/playback

## Data Flow Examples

### Text-Only Interaction
```
1. App creates Session with text modality
2. App calls client.update_session(session)
3. Client sends SessionUpdate event
4. App creates MessageItem with user text
5. App calls client.create_conversation_item(item)
6. Client sends ConversationItemCreate event
7. App calls client.create_response()
8. Client sends ResponseCreate event
9. Server responds with ResponseTextDelta events
10. App receives and displays streaming text
```

### Voice Interaction
```
1. App sets up audio devices and resampling
2. App creates Session with audio modality
3. Audio capture loop:
   a. Capture audio from microphone
   b. Resample to 24kHz
   c. Convert to PCM16 and base64 encode
   d. Send via client.append_input_audio_buffer()
4. Server processes audio and responds with:
   a. ResponseAudioDelta events (streaming audio)
   b. ResponseAudioTranscriptDelta events (text transcription)
5. App decodes audio and plays through speakers
```

## Error Handling Patterns

### Connection Errors
- WebSocket connection failures
- Authentication errors
- Network timeouts

### Audio Processing Errors
- Device not available
- Unsupported audio formats
- Buffer overflow/underflow

### API Errors
- Rate limiting
- Invalid requests
- Model errors

## Concurrency Model

The library uses tokio for async operations:

### Task Structure:
1. **Main Client Task**: Manages connection lifecycle
2. **Writer Task**: Sends client events to WebSocket
3. **Reader Task**: Receives and processes server events
4. **Audio Tasks**: Handle real-time audio processing

### Channel Communication:
- **MPSC Channels**: Client event sending (single producer, single consumer)
- **Broadcast Channels**: Server event distribution (single producer, multiple consumers)
- **Ring Buffers**: Real-time audio data sharing

## Performance Considerations

### Audio Latency
- Buffer sizes affect latency vs. stability tradeoff
- Resampling quality vs. performance
- Network latency to OpenAI servers

### Memory Usage
- Ring buffer sizing for audio streams
- Event queue sizing for high-throughput scenarios
- Connection pooling (not currently implemented)

### Error Recovery
- Automatic reconnection (not currently implemented)
- Audio stream recovery
- Graceful degradation

## Extension Points

### Custom Audio Processing
Implement your own audio pipeline by:
1. Using the base `Client` without utils
2. Implementing custom encoding/decoding
3. Managing your own device access

### Custom Event Handling
Extend event processing by:
1. Subscribing to server events
2. Implementing custom business logic
3. Managing conversation state

### Tool Integration
Add function calling by:
1. Defining tools in session configuration
2. Handling function call events
3. Implementing function execution
4. Returning results to the conversation

This architecture provides a solid foundation for real-time AI interactions while maintaining flexibility for custom implementations.