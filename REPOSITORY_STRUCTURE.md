# OpenAI Realtime RS - Complete Repository Structure Guide

This document provides a comprehensive overview of the `openai-realtime-rs` repository structure, explaining the purpose of every file and folder.

## Overview

`openai-realtime-rs` is a Rust library that provides a client implementation for OpenAI's Realtime API. It's structured as a Cargo workspace with multiple crates and supports real-time audio communication with OpenAI's models.

## Repository Structure

```
openai-realtime-rs/
├── .git/                           # Git version control directory
├── .idea/                          # JetBrains IDE configuration (ignored)
├── .gitignore                      # Git ignore rules
├── Cargo.lock                      # Dependency lock file (ignored for libraries)
├── Cargo.toml                      # Workspace root configuration
├── LICENSE                         # MIT License
├── README.md                       # Project overview and basic usage
├── REPOSITORY_STRUCTURE.md         # This documentation file
├── examples/                       # Example applications
│   ├── devices.rs                  # Audio device enumeration example
│   ├── hello.rs                    # Basic text-based interaction example
│   └── voice.rs                    # Full voice-to-voice example
├── src/                            # Main library source code
│   ├── lib.rs                      # Library entry point
│   ├── client.rs                   # Main client implementation
│   └── client/                     # Client module components
│       ├── config.rs               # Configuration and config builder
│       ├── consts.rs               # Constants and default values
│       ├── stats.rs                # Usage statistics tracking
│       └── utils.rs                # WebSocket request building utilities
├── types/                          # Type definitions crate
│   ├── Cargo.toml                  # Types crate configuration
│   ├── src/                        # Types source code
│   │   ├── lib.rs                  # Types library entry point
│   │   ├── audio.rs                # Audio-related type definitions
│   │   ├── events.rs               # Event type definitions
│   │   ├── session.rs              # Session configuration types
│   │   ├── tools.rs                # Tool/function definition types
│   │   ├── audio/                  # Audio module components
│   │   │   ├── consts.rs           # Audio constants
│   │   │   ├── transcription.rs    # Transcription configuration
│   │   │   └── turn_detection.rs   # Turn detection configuration
│   │   ├── content/                # Content-related types
│   │   │   ├── items.rs            # Conversation item types
│   │   │   ├── message.rs          # Message types
│   │   │   └── parts.rs            # Content part types
│   │   └── events/                 # Event type definitions
│   │       ├── client.rs           # Client-to-server events
│   │       ├── server.rs           # Server-to-client events
│   │       └── server/             # Server event sub-modules
│   └── target/                     # Build artifacts (ignored)
└── utils/                          # Utility functions crate
    ├── Cargo.toml                  # Utils crate configuration
    └── src/                        # Utils source code
        ├── lib.rs                  # Utils library entry point
        ├── audio.rs                # Audio processing utilities
        └── device.rs               # Audio device management
```

## Detailed File Analysis

### Root Level Files

#### Configuration Files
- **`Cargo.toml`**: Workspace configuration defining the main crate and two workspace members (`types` and `utils`). Configures features for TLS support and optional utils integration.
- **`.gitignore`**: Ignores build artifacts, IDE files, and environment files.
- **`LICENSE`**: MIT license (Copyright 2024 yykt).

#### Documentation
- **`README.md`**: Brief project overview with installation instructions and basic example usage.

### Main Library (`src/`)

#### Core Files
- **`lib.rs`**: Main library entry point that:
  - Re-exports the client functionality (`connect`, `Client`, `ServerRx`)
  - Re-exports types from the `openai-realtime-types` crate
  - Conditionally re-exports utils when the "utils" feature is enabled

- **`client.rs`**: The heart of the library containing the `Client` struct and WebSocket communication logic:
  - Manages WebSocket connections to OpenAI's Realtime API
  - Handles bidirectional event streaming (client→server and server→client)
  - Implements async message handling with tokio channels
  - Tracks usage statistics (tokens consumed)
  - Provides convenience methods for common operations (session updates, audio streaming, etc.)

#### Client Module Components (`src/client/`)
- **`config.rs`**: Configuration management:
  - `Config` struct with base URL, API key, and model settings
  - `ConfigBuilder` for fluent configuration building
  - Default values pointing to OpenAI's production endpoints
  
- **`consts.rs`**: Application constants:
  - API endpoint URLs
  - Default model name (`gpt-4o-realtime-preview-2024-10-01`)
  - HTTP header names for authentication

- **`stats.rs`**: Usage tracking:
  - `Stats` struct for tracking token consumption
  - Methods for updating and retrieving usage statistics
  - Thread-safe access via Mutex

- **`utils.rs`**: WebSocket utilities:
  - Builds authenticated WebSocket requests
  - Handles OpenAI-specific headers and authentication

### Type Definitions (`types/`)

The `types` crate defines all data structures used in the OpenAI Realtime API:

#### Core Type Files
- **`lib.rs`**: Types crate entry point, re-exporting all major types
- **`session.rs`**: Session configuration types:
  - `Session` struct with modalities, voice settings, audio formats, etc.
  - `SessionConfigurator` builder pattern for creating session configurations
  - Support for text/audio modalities, turn detection, tool integration

- **`tools.rs`**: Function/tool definition types for OpenAI function calling
- **`audio.rs`**: Audio-related type definitions and base64 encoding support

#### Event System (`types/src/events/`)
- **`events.rs`**: Main event enum definitions:
  - `ClientEvent` enum for client→server messages
  - `ServerEvent` enum for server→client messages
  - Comprehensive event types for all API operations

- **`client.rs`**: Client event definitions for:
  - Session updates
  - Audio buffer operations
  - Conversation item management
  - Response generation

- **`server.rs`**: Server event definitions for:
  - Session lifecycle events
  - Audio processing events
  - Response streaming events
  - Error handling

#### Content Types (`types/src/content/`)
- **`items.rs`**: Conversation item types (messages, function calls, etc.)
- **`message.rs`**: Message structure definitions with roles and content
- **`parts.rs`**: Content part types for different media types

#### Audio Configuration (`types/src/audio/`)
- **`consts.rs`**: Audio format constants and voice options
- **`transcription.rs`**: Audio transcription configuration
- **`turn_detection.rs`**: Voice activity detection settings

### Utility Functions (`utils/`)

The `utils` crate provides audio processing and device management utilities:

#### Core Utils Files
- **`lib.rs`**: Utils crate entry point
- **`audio.rs`**: Audio processing utilities:
  - PCM16 to/from base64 encoding/decoding
  - Sample rate conversion using rubato
  - Audio format conversion (f32 ↔ i16)
  - Ring buffer management for real-time audio
  - Constants for OpenAI's expected audio format (24kHz PCM16)

- **`device.rs`**: Audio device management:
  - Cross-platform audio device enumeration
  - Default device selection
  - Device capability querying
  - CPAL (Cross-Platform Audio Library) integration

### Examples (`examples/`)

- **`hello.rs`**: Simple text-based example:
  - Demonstrates basic connection and message sending
  - Shows both audio-enabled and text-only interactions
  - Minimal example for getting started

- **`voice.rs`**: Full voice-to-voice implementation:
  - Real-time audio capture from microphone
  - Audio resampling and format conversion
  - Bidirectional audio streaming with OpenAI
  - Audio playback of responses
  - Complex example showing full audio pipeline

- **`devices.rs`**: Audio device enumeration utility:
  - Lists available input/output audio devices
  - Shows device capabilities and configurations
  - Useful for debugging audio setup issues

## Architecture Overview

### Data Flow
1. **Client Connection**: WebSocket connection to OpenAI Realtime API
2. **Event Processing**: Bidirectional event streaming using tokio channels
3. **Audio Pipeline**: Microphone → Resampling → Base64 → API → Base64 → Resampling → Speakers
4. **Session Management**: Configuration and state management for conversations

### Key Design Patterns
- **Builder Pattern**: Used for configuration objects (`Session`, `Config`)
- **Event-Driven Architecture**: All communication via structured events
- **Async/Await**: Full async support using tokio
- **Type Safety**: Strong typing for all API interactions
- **Workspace Architecture**: Separation of concerns across multiple crates

### Dependencies
- **Core**: `tokio`, `tokio-tungstenite`, `serde`, `futures`
- **Audio**: `cpal`, `rubato`, `ringbuf`, `hound`
- **Security**: `secrecy` for API key handling
- **Utilities**: `base64`, `anyhow`, `tracing`

## Build and Development

### Building
```bash
cargo build                 # Build all workspace members
cargo build --example voice # Build specific example
```

### Features
- `default`: Includes `native-tls`
- `native-tls`: Uses native TLS implementation
- `rustls`: Uses pure Rust TLS implementation
- `utils`: Includes audio utility functions

### Testing
Currently no tests are implemented in the repository.

## Usage Patterns

1. **Simple Text Chat**: Use `hello.rs` as a template
2. **Voice Interaction**: Use `voice.rs` as a template  
3. **Device Discovery**: Use `devices.rs` to find audio devices
4. **Custom Configuration**: Use the builder patterns for sessions and config

This repository provides a solid foundation for building real-time AI voice applications with OpenAI's Realtime API in Rust.