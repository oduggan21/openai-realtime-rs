use crate::content::message::MessageItem;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Item {
    #[serde(rename = "message")]
    Message(MessageItem),
    #[serde(rename = "function_call")]
    FunctionCall(FunctionCallItem),
    #[serde(rename = "function_call_output")]
    FunctionCallOutput(FunctionCallOutputItem),
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ItemStatus {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "incomplete")]
    Incomplete,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct _Item {
    /// The unique ID of the item, Optional for client events
    pub id: Option<String>,

    /// The status of the item: "completed", "in_progress", "incomplete"
    pub status: Option<ItemStatus>,
}

impl Default for _Item {
    fn default() -> Self {
        Self {
            id: None,
            status: None,
        }
    }
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCallItem {
    #[serde(flatten)]
    item: _Item,
    /// The ID of the function call(for "function_call" items).
    call_id: Option<String>,

    /// The name of the function call(for "function_call" items).
    name: Option<String>,

    /// The arguments of the function call(for "function_call" items).
    arguments: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCallOutputItem {
    #[serde(flatten)]
    item: _Item,
    /// The output of the function call(for "function_call_output" items).
    output: Option<String>,
}