pub mod reviewer;
pub mod session_state;
pub mod topic;

/// Represents commands that the core logic (`FeynmanSession`) issues to the runtime.
///
/// This enum is the primary API for decoupling the session's decision-making
/// from the runtime's execution of side effects (like speaking text).
#[derive(Debug, Clone)]
pub enum Command {
    /// Command the runtime to speak the given text to the user.
    SpeakText(String),
    /// Command indicating the session (or a phase of it) is complete.
    /// Includes a final message for the user.
    SessionComplete(String),
}
