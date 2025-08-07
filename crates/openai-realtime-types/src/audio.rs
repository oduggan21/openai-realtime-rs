mod turn_detection;
mod transcription;
mod consts;

pub use turn_detection::{TurnDetection, ServerVadTurnDetection};
pub use transcription::InputAudioTranscription;
pub use consts::*;
/// Audio data encoded as base64
pub type Base64EncodedAudioBytes = String;
