
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum TurnDetection {
    #[serde(rename = "server_vad")]
    ServerVad(ServerVadTurnDetection),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerVadTurnDetection {
    /// Activation threshold for VAD(0.0 to 1.0).
    threshold: f32,

    /// Amount of audio to include before speech starts, in milliseconds
    prefix_padding_ms: i32,

    /// Duration fo silence to detect speech stop, in milliseconds
    silence_duration_ms: i32,
}

impl Default for TurnDetection {
    fn default() -> Self {
        Self::ServerVad(ServerVadTurnDetection::default())
    }
}

impl Default for ServerVadTurnDetection {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            prefix_padding_ms: 300,
            silence_duration_ms: 200,
        }
    }
}

impl ServerVadTurnDetection {
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn with_prefix_padding_ms(mut self, prefix_padding_ms: i32) -> Self {
        self.prefix_padding_ms = prefix_padding_ms;
        self
    }

    pub fn with_silence_duration_ms(mut self, silence_duration_ms: i32) -> Self {
        self.silence_duration_ms = silence_duration_ms;
        self
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn prefix_padding_ms(&self) -> i32 {
        self.prefix_padding_ms
    }

    pub fn silence_duration_ms(&self) -> i32 {
        self.silence_duration_ms
    }
}