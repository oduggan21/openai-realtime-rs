use std::fmt::Debug;
use crate::audio::TranscriptionModel;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioTranscription {
    /// Whether to enable audio transcription
    enabled: bool,
    /// The model to use for transcription: "whisper-1"
    model: TranscriptionModel,
}

impl Default for InputAudioTranscription {
    fn default() -> Self {
        Self {
            enabled: true,
            model: TranscriptionModel::Whisper,
        }
    }
}

impl InputAudioTranscription {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_model(mut self, model: TranscriptionModel) -> Self {
        self.model = model;
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn model(&self) -> TranscriptionModel {
        self.model.clone()
    }
}