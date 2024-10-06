use crate::audio::Base64EncodedAudioBytes;
use crate::content::items::{ItemStatus, _Item};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageItem {
    #[serde(flatten)]
    item: _Item,

    /// The role of the message sender: "user", "assistant", "system"
    role: MessageRole,

    /// The content of the message
    content: Vec<Content>,
}

impl MessageItem {
    pub fn builder() -> MessageItemBuilder {
        MessageItemBuilder::new()
    }
    
    pub fn id(&self) -> Option<String> {
        self.item.id.clone()
    }
    
    pub fn status(&self) -> Option<&str> {
        self.item.status.as_ref().map(|status| match status {
            ItemStatus::Completed => "completed",
            ItemStatus::InProgress => "in_progress",
            ItemStatus::Incomplete => "incomplete",
        })
    }
    
    pub fn role(&self) -> MessageRole {
        self.role.clone()
    }
    
    pub fn content(&self) -> Vec<Content> {
        self.content.clone()
    }
}

pub struct MessageItemBuilder {
    item: MessageItem,
}

impl Default for MessageItemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageItemBuilder {
    pub fn new() -> Self {
        Self {
            item: MessageItem {
                item: _Item::default(),
                role: MessageRole::User,
                content: Vec::new(),
            },
        }
    }
    
    pub fn with_id(mut self, id: &str) -> Self {
        self.item.item.id = Some(id.to_string());
        self
    }
    
    pub fn with_role(mut self, role: MessageRole) -> Self {
        self.item.role = role;
        self
    }
    
    pub fn with_input_text(mut self, text: &str) -> Self {
        self.item.content.push(Content::input_text(text));
        self
    }
    
    pub fn with_input_audio(mut self, audio: Base64EncodedAudioBytes) -> Self {
        self.item.content.push(Content::input_audio(audio));
        self
    }
    
    pub fn build(self) -> MessageItem {
        self.item
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum MessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "input_text")]
    InputText(InputTextContent),
    #[serde(rename = "input_audio")]
    InputAudio(InputAudioContent),
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "audio")]
    Audio(AudioContent),
}

impl Content {
    pub fn input_text(text: &str) -> Self {
        Content::InputText(InputTextContent::new(text))
    }
    
    pub fn input_audio(audio: Base64EncodedAudioBytes) -> Self {
        Content::InputAudio(InputAudioContent::new(audio))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputTextContent {
    text: String,
}

impl InputTextContent {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputAudioContent {
    audio: Base64EncodedAudioBytes,
}

impl InputAudioContent {
    pub fn new(audio: Base64EncodedAudioBytes) -> Self {
        Self {
            audio,
        }
    }

    pub fn audio(&self) -> Base64EncodedAudioBytes {
        self.audio.clone()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TextContent {
    text: String,
}

impl TextContent {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

   
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AudioContent {
    transcript: String,
}

impl AudioContent {
    pub fn new(text: &str) -> Self {
        Self {
            transcript: text.to_string(),
        }
    }

    pub fn transcript(&self) -> String {
        self.transcript.clone()
    }
}