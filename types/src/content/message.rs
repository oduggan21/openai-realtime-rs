use crate::audio::Base64EncodedAudioBytes;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputTextContent {
    text: String,
}

impl InputTextContent {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioContent {
    audio: Base64EncodedAudioBytes,
}

impl InputAudioContent {
    pub fn new(audio: Base64EncodedAudioBytes) -> Self {
        Self {
            audio,
        }
    }

    pub fn audio(&self) -> &Base64EncodedAudioBytes {
        &self.audio
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextContent {
    text: Option<String>,
    transcript: Option<String>,
}

impl TextContent {
    pub fn new(text: &str) -> Self {
        Self {
            text: Some(text.to_string()),
            transcript: None,
        }
    }

    pub fn new_transcript(transcript: &str) -> Self {
        Self {
            text: None,
            transcript: Some(transcript.to_string()),
        }
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn transcript(&self) -> Option<&str> {
        self.transcript.as_deref()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioContent {
    audio: Base64EncodedAudioBytes,
}

impl AudioContent {
    pub fn new(audio: Base64EncodedAudioBytes) -> Self {
        Self {
            audio,
        }
    }

    pub fn audio(&self) -> &Base64EncodedAudioBytes {
        &self.audio
    }
}