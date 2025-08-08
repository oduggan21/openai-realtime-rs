use crate::reviewer::Reviewer;
use crate::topic::{SubTopic, SubTopicList};
use anyhow::Result;
use rmcp::tool_handler;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

// --- Agent State ---

/// Represents the persistent state of the Feynman teaching session.
/// This struct holds the progress of the user's teaching, including which
/// subtopics have been covered, which are still incomplete, and the overall topic.
pub struct FeynmanAgent {
    pub main_topic: String,
    pub subtopic_list: SubTopicList,
    pub covered_subtopics: HashMap<String, SubTopic>,
    pub incomplete_subtopics: HashMap<String, SubTopic>,
}

impl FeynmanAgent {
    /// Creates a new FeynmanAgent for a given topic and its subtopics.
    pub fn new(main_topic: String, subtopic_list: SubTopicList) -> Self {
        // Initially, all subtopics are considered incomplete.
        let incomplete_subtopics = subtopic_list
            .subtopics
            .iter()
            .map(|st| (st.name.clone(), st.clone()))
            .collect();

        Self {
            main_topic,
            subtopic_list,
            covered_subtopics: HashMap::new(),
            incomplete_subtopics,
        }
    }
}

// --- Service and Handler Implementation ---

/// The main service that implements the MCP ServerHandler.
/// It holds the agent's state and dependencies (like the reviewer), and exposes
/// its capabilities as MCP tools.
pub struct FeynmanService {
    pub agent_state: Arc<tokio::sync::Mutex<FeynmanAgent>>,
    pub reviewer: Arc<dyn Reviewer>,
    tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for FeynmanService {
    /// Provides information about the server, including its capabilities.
    /// This tells the client that this server supports tools.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "A teaching assistant agent that helps you practice the Feynman technique.".into(),
            ),
            ..Default::default()
        }
    }
}

// --- Argument Structs for Tools ---

#[derive(Deserialize, JsonSchema)]
pub struct SendMessageArgs {
    pub text: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AnalyzeTopicArgs {
    pub segment: String,
    pub subtopics: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GenerateSubtopicsArgs {
    pub topic: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AnalyzeAnswerArgs {
    pub question: String,
    pub answer: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct LooksLikeTopicChangeArgs {
    pub context_buffer: String,
    pub new_segment: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CheckAnswerSatisfiesQuestionArgs {
    pub segment: String,
    pub question: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AnalyzeLastExplainedContextArgs {
    pub segment: String,
    pub main_topic: String,
    pub subtopic_list: Vec<String>,
}

// --- Tool Implementations ---

#[tool_router]
impl FeynmanService {
    pub fn new(
        agent_state: Arc<tokio::sync::Mutex<FeynmanAgent>>,
        reviewer: Arc<dyn Reviewer>,
    ) -> Self {
        Self {
            agent_state,
            reviewer,
            tool_router: Self::tool_router(),
        }
    }

    /// Processes a user's message, acting as the main entry point for the agent's logic loop.
    #[tool(description = "Send a message to the Feynman agent to continue the lesson.")]
    pub async fn send_message(&self, args: Parameters<SendMessageArgs>) -> Result<String, String> {
        // This tool now contains the "agent loop" logic.
        // It constructs a prompt and simulates an LLM call.
        // In a real scenario, this would involve a call to an LLM client,
        // which would then decide which other tools to call.

        let agent = self.agent_state.lock().await;

        let covered_list = agent
            .covered_subtopics
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let incomplete_list = agent
            .incomplete_subtopics
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        let system_prompt = format!(
            r#"You are a student learning about the topic: "{}". Your goal is to understand it thoroughly by asking clarifying questions.
The teacher (the user) will explain concepts to you. Your job is to use the available tools to analyze their explanations.

Current user message: "{}"

Session Progress:
- Main Topic: {}
- Subtopics fully covered: {}
- Subtopics partially covered or not started: {}

Instructions:
1. Listen to the user's explanation.
2. Use the `analyze_topic` tool to check if their explanation covers the definition, mechanism, and an example for any mentioned subtopics.
3. If the analysis shows missing parts, the tool will provide questions. Ask the user these questions to fill the gaps.
4. If the user answers a question, use the `analyze_answer` tool to see if the answer is correct.
5. Continue this process until all subtopics are fully covered.
6. Be encouraging and curious.
"#,
            agent.main_topic,
            args.0.text,
            agent.main_topic,
            if covered_list.is_empty() {
                "None yet"
            } else {
                &covered_list
            },
            if incomplete_list.is_empty() {
                "None"
            } else {
                &incomplete_list
            }
        );

        tracing::info!("--- SIMULATED LLM CALL ---");
        tracing::info!("Generated Prompt:\n{}", system_prompt);
        tracing::info!("--------------------------");

        let placeholder_response = "That's interesting! I'm analyzing your explanation now. Could you tell me more about one of the subtopics?".to_string();

        // Simply return the string. rmcp will handle wrapping it in the correct Content type.
        Ok(placeholder_response)
    }

    /// Analyzes a user's explanation of one or more subtopics to check for completeness.
    #[tool]
    pub async fn analyze_topic(
        &self,
        args: Parameters<AnalyzeTopicArgs>,
    ) -> Result<rmcp::Json<serde_json::Value>, String> {
        let subtopics: Vec<SubTopic> = args.0.subtopics.into_iter().map(SubTopic::new).collect();
        let result_str = self
            .reviewer
            .analyze_topic(&args.0.segment, &subtopics)
            .await
            .map_err(|e| e.to_string())?;
        let json_val: serde_json::Value =
            serde_json::from_str(&result_str).map_err(|e| e.to_string())?;
        Ok(rmcp::Json(json_val))
    }

    /// Generates a list of key subtopics for a given main topic.
    #[tool]
    pub async fn generate_subtopics(
        &self,
        args: Parameters<GenerateSubtopicsArgs>,
    ) -> Result<rmcp::Json<serde_json::Value>, String> {
        let result = self
            .reviewer
            .generate_subtopics(&args.0.topic)
            .await
            .map_err(|e| e.to_string())?;
        let json_val = serde_json::to_value(result).map_err(|e| e.to_string())?;
        Ok(rmcp::Json(json_val))
    }

    /// Analyzes a user's answer to a specific question for correctness.
    #[tool]
    pub async fn analyze_answer(
        &self,
        args: Parameters<AnalyzeAnswerArgs>,
    ) -> Result<rmcp::Json<serde_json::Value>, String> {
        let result = self
            .reviewer
            .analyze_answer(&args.0.question, &args.0.answer)
            .await
            .map_err(|e| e.to_string())?;
        let json_val = serde_json::to_value(result).map_err(|e| e.to_string())?;
        Ok(rmcp::Json(json_val))
    }

    /// Checks if a new segment of user speech indicates a change in topic.
    #[tool]
    pub async fn looks_like_topic_change(
        &self,
        args: Parameters<LooksLikeTopicChangeArgs>,
    ) -> Result<rmcp::Json<serde_json::Value>, String> {
        let result_str = self
            .reviewer
            .looks_like_topic_change(&args.0.context_buffer, &args.0.new_segment)
            .await
            .map_err(|e| e.to_string())?;
        let json_val: serde_json::Value =
            serde_json::from_str(&result_str).map_err(|e| e.to_string())?;
        Ok(rmcp::Json(json_val))
    }

    /// Checks if a user's explanation adequately answers a specific question.
    #[tool]
    pub async fn check_answer_satisfies_question(
        &self,
        args: Parameters<CheckAnswerSatisfiesQuestionArgs>,
    ) -> Result<rmcp::Json<serde_json::Value>, String> {
        let result = self
            .reviewer
            .check_answer_satisfies_question(&args.0.segment, &args.0.question)
            .await
            .map_err(|e| e.to_string())?;
        let json_val = serde_json::to_value(result).map_err(|e| e.to_string())?;
        Ok(rmcp::Json(json_val))
    }

    /// Provides a summary or feedback on the last piece of user explanation.
    #[tool]
    pub async fn analyze_last_explained_context(
        &self,
        args: Parameters<AnalyzeLastExplainedContextArgs>,
    ) -> Result<rmcp::Json<serde_json::Value>, String> {
        let result = self
            .reviewer
            .analyze_last_explained_context(
                &args.0.segment,
                &args.0.main_topic,
                &args.0.subtopic_list,
            )
            .await
            .map_err(|e| e.to_string())?;
        let json_val = serde_json::to_value(result).map_err(|e| e.to_string())?;
        Ok(rmcp::Json(json_val))
    }
}
