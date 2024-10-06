use crate::audio::{AudioFormat, InputAudioTranscription, TranscriptionModel, TurnDetection, Voice};
use crate::tools::{Tool, ToolChoice};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    /// The set of modalities the model can respond with. To disable audio, set this to ["text"].
    /// To enable audio, set this to ["text", "audio"].
    modalities: Vec<String>,

    /// The default system instructions prepended to model calls.
    instructions: Option<String>,

    /// The voice the model uses to respond. Cannot be changed once the model has responded with audio at least one.
    /// ex: "alloy"
    voice: Option<Voice>,

    /// The format of input audio. Options are "pcm16", "g711_ulaw", "g711_alaw".
    input_audio_format: Option<AudioFormat>,

    /// The format of output audio. Options are "pcm16", "g711_ulaw", "g711_alaw".
    output_audio_format: Option<AudioFormat>,

    /// Configuration for input audio transcription. Can be set to null to turn off
    input_audio_transcription: Option<InputAudioTranscription>,

    /// Configuration for turn detection. Can be set to null to turn off
    turn_detection: Option<TurnDetection>,

    /// Tools(Functions) available to the model.
    tools: Vec<Tool>,

    /// How the model chooses tools. Options are "auto", "none", "required", or specify a function.
    tool_choice: Option<ToolChoice>,

    /// Sampling temperature for the model.
    temperature: f32,

    /// Maximum number of output tokens. Use "inf" for infinity.
    /// "inf" or number
    max_output_tokens: Option<MaxOutputTokens>,
}

impl Session {}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MaxOutputTokens {
    Number(i32),
    Infinity(String),
}


pub struct SessionConfigurator {
    session: Session,
}

impl SessionConfigurator {
    pub fn new() -> Self {
        Self {
            session: Session {
                modalities: vec!["text".to_string(), "audio".to_string()],
                instructions: None,
                voice: None,
                input_audio_format: None,
                output_audio_format: None,
                input_audio_transcription: None,
                turn_detection: None,
                tools: vec![],
                tool_choice: Some(ToolChoice::Auto),
                temperature: 0.0,
                max_output_tokens: None,
            }
        }
    }

    pub fn with_modalities(mut self, modalities: Vec<String>) -> Self {
        self.session.modalities = modalities;
        self
    }

    pub fn with_modalities_disable_audio(mut self) -> Self {
        self.session.modalities = vec!["text".to_string()];
        self
    }

    pub fn with_modalities_enable_audio(mut self) -> Self {
        self.session.modalities = vec!["text".to_string(), "audio".to_string()];
        self
    }

    pub fn with_instructions(mut self, instructions: &str) -> Self {
        self.session.instructions = Some(instructions.to_string());
        self
    }

    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.session.voice = Some(voice);
        self
    }

    pub fn with_input_audio_format(mut self, format: AudioFormat) -> Self {
        self.session.input_audio_format = Some(format);
        self
    }

    pub fn with_output_audio_format(mut self, format: AudioFormat) -> Self {
        self.session.output_audio_format = Some(format);
        self
    }

    pub fn with_input_audio_transcription(mut self, input_audio_transcription: InputAudioTranscription) -> Self {
        self.session.input_audio_transcription = Some(input_audio_transcription);
        self
    }

    pub fn with_input_audio_transcription_disable(mut self) -> Self {
        self.session.input_audio_transcription = None;
        self
    }

    pub fn with_input_audio_transcription_enable(mut self, model: TranscriptionModel) -> Self {
        self.session.input_audio_transcription = Some(InputAudioTranscription::new()
            .with_model(model));
        self
    }

    pub fn with_turn_detection_enable(mut self, turn_detection: TurnDetection) -> Self {
        self.session.turn_detection = Some(turn_detection);
        self
    }

    pub fn with_turn_detection_disable(mut self) -> Self {
        self.session.turn_detection = None;
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.session.tools = tools;
        self
    }

    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.session.tool_choice = Some(tool_choice);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.session.temperature = temperature;
        self
    }

    pub fn with_max_output_tokens(mut self, max_output_tokens: MaxOutputTokens) -> Self {
        self.session.max_output_tokens = Some(max_output_tokens);
        self
    }

    pub fn build(self) -> Session {
        self.session
    }
}