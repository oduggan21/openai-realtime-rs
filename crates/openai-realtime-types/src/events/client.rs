use crate::audio::Base64EncodedAudioBytes;
use crate::Item;
use crate::session::Session;


/// `session.update` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionUpdateEvent {
    event_id: Option<String>,

    /// The session configuration to update
    session: Session,
}

impl SessionUpdateEvent {
    pub fn new(session: Session) -> Self {
        Self {
            event_id: None,
            session,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
}

/// `input_audio_buffer.append` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferAppendEvent {
    event_id: Option<String>,


    /// The audio data to append to the buffer
    audio: Base64EncodedAudioBytes,
}

impl InputAudioBufferAppendEvent {
    pub fn new(audio: Base64EncodedAudioBytes) -> Self {
        Self {
            event_id: None,
            audio,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }

    pub fn audio(&self) -> &Base64EncodedAudioBytes {
        &self.audio
    }
}

//tells us when we are done sendinging audio
/// `input_audio_buffer.commit` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferCommitEvent {
    event_id: Option<String>,
}

impl InputAudioBufferCommitEvent {
    pub fn new() -> Self {
        Self {
            event_id: None,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
}

/// `input_audio_buffer.clear` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferClearEvent {
    event_id: Option<String>,
}

impl InputAudioBufferClearEvent {
    pub fn new() -> Self {
        Self {
            event_id: None,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
}

/// `conversation.item.create` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemCreateEvent {
    event_id: Option<String>,


    /// The ID of the preceding item after which the new item will be inserted
    pub previous_item_id: Option<String>,
    /// The item to add to the conversation
    pub item: Item,
}

impl ConversationItemCreateEvent {
    pub fn new(item: Item) -> Self {
        Self {
            event_id: None,
            previous_item_id: None,
            item,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
    pub fn with_previous_item_id(mut self, previous_item_id: &str) -> Self {
        self.previous_item_id = Some(previous_item_id.to_string());
        self
    }
}

/// `conversation.item.truncate` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemTruncateEvent {
    event_id: Option<String>,


    /// The ID of the assistant message item to truncate.
    pub item_id: String,
    /// The index of the content part to truncate
    pub content_index: i32,
    /// inclusive duration up to which audio is truncated, in milliseconds
    pub audio_end_ms: i32,
}

impl ConversationItemTruncateEvent {
    pub fn new(item_id: &str, content_index: i32, audio_end_ms: i32) -> Self {
        Self {
            event_id: None,
            item_id: item_id.to_string(),
            content_index,
            audio_end_ms,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
}

/// `conversation.item.delete` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemDeleteEvent {
    event_id: Option<String>,


    /// The ID of the item to delete
    pub item_id: String,
}

impl ConversationItemDeleteEvent {
    pub fn new(item_id: &str) -> Self {
        Self {
            event_id: None,
            item_id: item_id.to_string(),
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
}

/// `response.create` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseCreateEvent {
    event_id: Option<String>,


    /// Configuration for the response
    response: Option<Session>,
}

impl Default for ResponseCreateEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseCreateEvent {
    pub fn new() -> Self {
        Self {
            event_id: None,
            response: None,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
    pub fn with_update_session(mut self, response: Session) -> Self {
        self.response = Some(response);
        self
    }
}

/// `response.cancel` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseCancelEvent {
    event_id: Option<String>,

}

impl ResponseCancelEvent {
    pub fn new() -> Self {
        Self {
            event_id: None,
        }
    }
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
}