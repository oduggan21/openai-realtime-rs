#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorDetails {
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
    message: String,
    param: Option<String>,
    event_id: Option<String>,
}

impl ErrorDetails {
    pub fn error_type(&self) -> &str {
        &self.error_type
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn param(&self) -> Option<&str> {
        self.param.as_deref()
    }

    pub fn event_id(&self) -> Option<&str> {
        self.event_id.as_deref()
    }
}

impl ErrorDetails {
    pub fn new(error_type: &str, message: &str) -> Self {
        Self {
            error_type: error_type.to_string(),
            code: None,
            message: message.to_string(),
            param: None,
            event_id: None,
        }
    }

    pub fn with_code(mut self, code: &str) -> Self {
        self.code = Some(code.to_string());
        self
    }

    pub fn with_param(mut self, param: &str) -> Self {
        self.param = Some(param.to_string());
        self
    }

    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = Some(event_id.to_string());
        self
    }
}

