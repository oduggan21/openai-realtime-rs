use std::fmt::Debug;
use crate::audio::TranscriptionModel;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioTranscription {
    /// Whether to enable audio transcription
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    /// The model to use for transcription: "whisper-1"
    model: TranscriptionModel,
}

impl Default for InputAudioTranscription {
    fn default() -> Self {
        Self {
            enabled: None,
            model: TranscriptionModel::Whisper,
        }
    }
}

impl InputAudioTranscription {
    pub fn new() -> Self {
        Self::default()
    }
    

    pub fn with_model(mut self, model: TranscriptionModel) -> Self {
        self.model = model;
        self
    }
    pub fn with_enabled(mut self, enabled: bool) -> Self{
        self.enabled = Some(enabled);
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled.map_or(true, |x| x)
    }

    pub fn model(&self) -> TranscriptionModel {
        self.model.clone()
    }
}