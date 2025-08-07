use crate::audio::Base64EncodedAudioBytes;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "audio")]
    Audio(AudioTranscriptPart),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextPart {
    text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioTranscriptPart {
    transcript: String,
}