#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Voice {
    #[serde(rename = "alloy")]
    Alloy,
    #[serde(rename = "echo")]
    Echo,
    #[serde(rename = "fable")]
    Fable,
    #[serde(rename = "onyx")]
    Onyx,
    #[serde(rename = "nova")]
    Nova,
    #[serde(rename = "shimmer")]
    Shimmer,
    Custom(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum AudioFormat {
    #[serde(rename = "pcm16")]
    Pcm16,
    #[serde(rename = "g711_ulaw")]
    Mulaw,
    #[serde(rename = "g711_alaw")]
    Alaw,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum TranscriptionModel {
    #[serde(rename = "whisper-1")]
    Whisper,
    Custom(String),
}