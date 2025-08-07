pub mod reviewer;
pub mod session_state;
pub mod topic;

pub enum Input {
    Audio(Vec<f32>),
    Initialize(),
    Initialized(),
    AISpeaking(),
    AISpeakingDone(),
    CreateConversationItem(openai_realtime::types::Item),
}
