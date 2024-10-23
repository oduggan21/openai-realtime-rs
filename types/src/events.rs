pub mod client;
mod server;

use client::*;
use server::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate(SessionUpdateEvent),
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend(InputAudioBufferAppendEvent),
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit(InputAudioBufferCommitEvent),
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear(InputAudioBufferClearEvent),
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate(ConversationItemCreateEvent),
    #[serde(rename = "conversation.item.truncate")]
    ConversationItemTruncate(ConversationItemTruncateEvent),
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete(ConversationItemDeleteEvent),
    #[serde(rename = "response.create")]
    ResponseCreate(ResponseCreateEvent),
    #[serde(rename = "response.cancel")]
    ResponseCancel(ResponseCancelEvent),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "close")]
    Close {
        reason: Option<String>,
    },
    #[serde(rename = "error")]
    Error(ErrorEvent),
    #[serde(rename = "session.created")]
    SessionCreated(SessionCreatedEvent),
    #[serde(rename = "session.updated")]
    SessionUpdated(SessionUpdatedEvent),
    #[serde(rename = "conversation.created")]
    ConversationCreated(ConversationCreatedEvent),
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted(InputAudioBufferCommittedEvent),
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared(InputAudioBufferClearedEvent),
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted(InputAudioBufferSpeechStartedEvent),
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped(InputAudioBufferSpeechStoppedEvent),
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated(ConversationItemCreatedEvent),
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    ConversationItemInputAudioTranscriptionCompleted(ConversationItemInputAudioTranscriptionCompletedEvent),
    #[serde(rename = "conversation.item.input_audio_transcription.failed")]
    ConversationItemInputAudioTranscriptionFailed(ConversationItemInputAudioTranscriptionFailedEvent),
    #[serde(rename = "conversation.item.truncated")]
    ConversationItemTruncated(ConversationItemTruncateEvent),
    #[serde(rename = "conversation.item.deleted")]
    ConversationItemDeleted(ConversationItemDeleteEvent),
    #[serde(rename = "response.created")]
    ResponseCreated(ResponseCreatedEvent),
    #[serde(rename = "response.done")]
    ResponseDone(ResponseDoneEvent),
    #[serde(rename = "response.output_item.added")]
    ResponseOutputItemAdded(ResponseOutputItemAddedEvent),
    #[serde(rename = "response.output_item.done")]
    ResponseOutputItemDone(ResponseOutputItemDoneEvent),
    #[serde(rename = "response.content_part.added")]
    ResponseContentPartAdded(ResponseContentPartAddedEvent),
    #[serde(rename = "response.content_part.done")]
    ResponseContentPartDone(ResponseContentPartDoneEvent),
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta(ResponseTextDeltaEvent),
    #[serde(rename = "response.text.done")]
    ResponseTextDone(ResponseTextDoneEvent),
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta(ResponseAudioTranscriptDeltaEvent),
    #[serde(rename = "response.audio_transcript.done")]
    ResponseAudioTranscriptDone(ResponseAudioTranscriptDoneEvent),
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta(ResponseAudioDeltaEvent),
    #[serde(rename = "response.audio.done")]
    ResponseAudioDone(ResponseAudioDoneEvent),
    #[serde(rename = "response.function_call_arguments.delta")]
    ResponseFunctionCallArgumentsDelta(ResponseFunctionCallArgumentsDeltaEvent),
    #[serde(rename = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone(ResponseFunctionCallArgumentsDoneEvent),
    #[serde(rename = "rate_limits.updated")]
    RateLimitsUpdated(RateLimitsUpdatedEvent),
}