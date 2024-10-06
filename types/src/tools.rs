#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "required")]
    Required,
    Specific(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Tool {
    #[serde(rename = "function")]
    Function(FunctionTool),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionTool {
    /// The name of the function
    name: String,

    /// The description of the function
    description: String,

    /// The parameters of the function in JSON Schema format
    parameters: serde_json::Value,
}

impl FunctionTool {
    pub fn new(name: String, description: String, parameters: serde_json::Value) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn parameters(&self) -> &serde_json::Value {
        &self.parameters
    }
}