#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum TurnDetection {
    #[serde(rename = "server_vad")]
    ServerVad(ServerVadTurnDetection),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ServerVadTurnDetection {
    /// Activation threshold for VAD(0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold: Option<f32>,

    /// Amount of audio to include before speech starts, in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix_padding_ms: Option<i32>,

    /// Duration of silence to detect speech stop, in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    silence_duration_ms: Option<i32>,

    /// Whether the model should interrupt its response when the user starts speaking.
    #[serde(skip_serializing_if = "Option::is_none")]
    interrupt_response: Option<bool>,

    /// Whether to automatically create a response when the user stops speaking.
    #[serde(skip_serializing_if = "Option::is_none")]
    create_response: Option<bool>,
}

impl Default for TurnDetection {
    fn default() -> Self {
        Self::ServerVad(ServerVadTurnDetection::default())
    }
}

impl ServerVadTurnDetection {
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = Some(threshold);
        self
    }

    pub fn with_prefix_padding_ms(mut self, prefix_padding_ms: i32) -> Self {
        self.prefix_padding_ms = Some(prefix_padding_ms);
        self
    }

    pub fn with_silence_duration_ms(mut self, silence_duration_ms: i32) -> Self {
        self.silence_duration_ms = Some(silence_duration_ms);
        self
    }

    pub fn with_interrupt_response(mut self, interrupt: bool) -> Self {
        self.interrupt_response = Some(interrupt);
        self
    }

    pub fn with_create_response(mut self, create: bool) -> Self {
        self.create_response = Some(create);
        self
    }

    pub fn threshold(&self) -> Option<f32> {
        self.threshold
    }

    pub fn prefix_padding_ms(&self) -> Option<i32> {
        self.prefix_padding_ms
    }

    pub fn silence_duration_ms(&self) -> Option<i32> {
        self.silence_duration_ms
    }

    pub fn interrupt_response(&self) -> Option<bool> {
        self.interrupt_response
    }

    pub fn create_response(&self) -> Option<bool> {
        self.create_response
    }
}
