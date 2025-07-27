mod resources;
mod error;

use resources::*;
use crate::ContentPart;
use crate::events::server::error::ErrorDetails;

/// `error` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorEvent {
    event_id: String,

    /// Details about the error
    error: error::ErrorDetails,
}

impl ErrorEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn error(&self) -> error::ErrorDetails {
        self.error.clone()
    }
}

/// `session.created` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionCreatedEvent {
    event_id: String,
    /// The session resource
    session: resources::SessionResource,
}

impl SessionCreatedEvent {
    pub fn event_id(&self) -> &str {
        self.event_id.as_str()
    }

    pub fn session(&self) -> &resources::SessionResource {
        &self.session
    }
}


/// `session.updated` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionUpdatedEvent {
    event_id: String,

    /// The updated session resource
    session: resources::SessionResource,
}
impl SessionUpdatedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn session(&self) -> resources::SessionResource {
        self.session.clone()
    }
}

/// `conversation.created` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationCreatedEvent {
    event_id: String,

    /// The conversation resource
    conversation: resources::ConversationResource,
}

impl ConversationCreatedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn conversation(&self) -> ConversationResource {
        self.conversation.clone()
    }
}


/// `input_audio_buffer.commited` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferCommittedEvent {
    event_id: String,


    /// The ID of the preceding item after which the new item will be inserted
    previous_item_id: Option<String>,
    /// The ID of the user message item that will be created
    item_id: String,
}

impl InputAudioBufferCommittedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn previous_item_id(&self) -> Option<&str> {
        self.previous_item_id.as_deref()
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }
}

/// `input_audio_buffer.cleared` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferClearedEvent {
    event_id: String,

}
impl InputAudioBufferClearedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
}

/// `input_audio_buffer.speech_started` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferSpeechStartedEvent {
    event_id: String,

    /// Milliseconds since the session started when speech was detected
    audio_start_ms: i32,
    /// The ID of the user message item that will be created when speech stops
    item_id: String,
}

impl InputAudioBufferSpeechStartedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn audio_start_ms(&self) -> i32 {
        self.audio_start_ms
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }
}

/// `input_audio_buffer.speech_stopped` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputAudioBufferSpeechStoppedEvent {
    event_id: String,

    /// Milliseconds since the session started when speech stopped
    audio_end_ms: i32,
    /// The ID of the user message item that will be created
    item_id: String,
}

impl InputAudioBufferSpeechStoppedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn audio_end_ms(&self) -> i32 {
        self.audio_end_ms
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }
}

/// `conversation.item.created` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemCreatedEvent {
    event_id: String,

    /// The ID of the preceding item
    previous_item_id: Option<String>,
    /// The item that was created
    item: ItemResource,
}

impl ConversationItemCreatedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn previous_item_id(&self) -> Option<&str> {
        self.previous_item_id.as_deref()
    }

    pub fn item(&self) -> ItemResource {
        self.item.clone()
    }
}

/// `conversation.item.input_audio_transcription.completed` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemInputAudioTranscriptionCompletedEvent {
    event_id: String,

    /// The ID of the user message item
    item_id: String,

    /// The index of the content part containing the audio
    content_index: i32,

    /// The transcribed text
    transcript: String,
}

impl ConversationItemInputAudioTranscriptionCompletedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TranscriptionLogprob {
    bytes: Vec<u8>,
    logprob: f64,
    token: String,
}

impl TranscriptionLogprob{
    pub fn bytes(&self) -> &Vec<u8>{
        &self.bytes
    }
    pub fn logprob(&self) -> f64{
        self.logprob
    }
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// 'conversation.item.input_audio_transcription.delta' event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemInputAudioTranscriptionDelta{
    event_id: String,

    item_id: String, 

    content_index: i32,

    delta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<Vec<TranscriptionLogprob>>,
}

impl ConversationItemInputAudioTranscriptionDelta{
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    pub fn item_id(&self) -> &str {
        &self.item_id
    }
    pub fn content_index(&self) -> i32 {
        self.content_index
    }
    pub fn delta(&self) -> &str {
        &self.delta
    }
    pub fn logprobs(&self) -> Option<&Vec<TranscriptionLogprob>> {
        self.logprobs.as_ref()
    }

}

/// `conversation.item.input_audio_transcription.failed` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemInputAudioTranscriptionFailedEvent {
    event_id: String,


    /// The ID of the user message item
    item_id: String,

    /// The index of the content part containing the audio
    content_index: i32,

    /// Details of the transcription error
    error: ErrorDetails,
}

impl ConversationItemInputAudioTranscriptionFailedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn error(&self) -> &ErrorDetails {
        &self.error
    }
}

/// `conversation.item.truncated` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemTruncatedEvent {
    event_id: String,

    /// The ID of the assistant message item that was truncated
    item_id: String,
    /// The index of the content part that was truncated
    content_index: i32,
    /// The duration up to which the audio was truncated, in milliseconds
    audio_end_ms: i32,
}

impl ConversationItemTruncatedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn audio_end_ms(&self) -> i32 {
        self.audio_end_ms
    }
}

/// `conversation.item.deleted` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationItemDeletedEvent {
    event_id: String,

    /// The ID of the item that was deleted
    item_id: String,
}

impl ConversationItemDeletedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }
}

/// `response.created` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseCreatedEvent {
    event_id: String,

    /// The response resource
    response: ResponseResource,
}

impl ResponseCreatedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response(&self) -> &ResponseResource {
        &self.response
    }
}

/// `response.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseDoneEvent {
    event_id: String,

    /// The response resource
    response: ResponseResource,
}

impl ResponseDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response(&self) -> &ResponseResource {
        &self.response
    }
}

/// `response.output_item.added` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseOutputItemAddedEvent {
    event_id: String,

    /// The ID of the response to which the item belongs
    response_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The item that was added
    item: ItemResource,
}

impl ResponseOutputItemAddedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn item(&self) -> &ItemResource {
        &self.item
    }
}

/// `response.output_item.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseOutputItemDoneEvent {
    event_id: String,

    /// The ID of the response to which the item belongs
    response_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The completed item
    item: ItemResource,
}

impl ResponseOutputItemDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn item(&self) -> &ItemResource {
        &self.item
    }
}

/// `response.content_part.added` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseContentPartAddedEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item to which the content part was added
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The content part that was added
    part: ContentPart,
}

impl ResponseContentPartAddedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn part(&self) -> &ContentPart {
        &self.part
    }
}

/// `response.content_part.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseContentPartDoneEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item to which the content part was added
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The completed content part
    part: ContentPart,
}

impl ResponseContentPartDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn part(&self) -> ContentPart {
        self.part.clone()
    }
}

/// `response.audio.delta` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseTextDeltaEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The delta in the text content
    delta: String,
}

impl ResponseTextDeltaEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn delta(&self) -> &str {
        &self.delta
    }
}

/// `response.text.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseTextDoneEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The completed text content
    text: String,
}

impl ResponseTextDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// `response.audio_transcript.delta` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseAudioTranscriptDeltaEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The delta in the audio transcript
    delta: String,
}

impl ResponseAudioTranscriptDeltaEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn delta(&self) -> &str {
        &self.delta
    }
}

/// `response.audio_transcript.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseAudioTranscriptDoneEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The completed audio transcript
    transcript: String,
}

impl ResponseAudioTranscriptDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }
}

/// `response.audio.delta` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseAudioDeltaEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
    /// The delta in the audio content
    delta: String,
}

impl ResponseAudioDeltaEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }

    pub fn delta(&self) -> &str {
        &self.delta
    }
}

/// `response.audio.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseAudioDoneEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The index of the content part in the item's content array
    content_index: i32,
}

impl ResponseAudioDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn content_index(&self) -> i32 {
        self.content_index
    }
}


/// `response.function_call_arguments.delta` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseFunctionCallArgumentsDeltaEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the function call item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The ID of the function call
    call_id: String,
    /// The delta in the function calling arguments
    delta: String,
}

impl ResponseFunctionCallArgumentsDeltaEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    pub fn delta(&self) -> &str {
        &self.delta
    }
}

/// `response.function_call_arguments.done` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseFunctionCallArgumentsDoneEvent {
    event_id: String,

    /// The ID of the response
    response_id: String,
    /// The ID of the function call item
    item_id: String,
    /// The index of the output item in the response
    output_index: i32,
    /// The ID of the function call
    call_id: String,
    /// The completed function calling arguments
    arguments: String,
}

impl ResponseFunctionCallArgumentsDoneEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn response_id(&self) -> &str {
        &self.response_id
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn output_index(&self) -> i32 {
        self.output_index
    }

    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    pub fn arguments(&self) -> &str {
        &self.arguments
    }
}

/// `rate_limits.updated` event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitsUpdatedEvent {
    event_id: String,

    /// List of rate limit information
    rate_limits: Vec<RateLimitInformation>,
}

impl RateLimitsUpdatedEvent {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn rate_limits(&self) -> &[RateLimitInformation] {
        &self.rate_limits
    }
}