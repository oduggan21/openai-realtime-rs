pub mod reviewer;
pub mod session_state;
pub mod topic;
pub mod generic_types;
pub mod realtime_api;

/// Represents commands that the core logic (`FeynmanSession`) issues to the runtime.
///
/// This enum is the primary API for decoupling the session's decision-making
/// from the runtime's execution of side effects (like speaking text).
#[derive(Debug, Clone)]
pub enum Command {
    /// Command the runtime to speak the given text to the user.
    SpeakText(String),
    /// Command indicating the session is complete, with a final message.
    SessionComplete(String),
}

// NOTE This Input enum is specific to the `feynman-service` runtime.
// It is temporarily placed here to integrate 6c6063f
// A future refactor should move this into the `feynman-service` crate itself.
pub enum Input {
    Audio(Vec<f32>),
    Initialize(),
    Initialized(),
    AISpeaking(),
    AISpeakingDone(),
    CreateSpokenResponse(String),
}