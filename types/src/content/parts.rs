use crate::audio::Base64EncodedAudioBytes;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ContentPart {
    Text(String),
    Audio(Base64EncodedAudioBytes),
}