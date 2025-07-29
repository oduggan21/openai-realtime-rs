use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct TopicChange{
    topic_change: bool,
    new_topic: Option<String>
}

#[derive(Debug, Deserialize)]
struct LlmResponse{
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice{
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message{
    content: String,
}

pub struct ReviewerClient{ 
    client: Client,
    api_key: String,
    model: String,
}

impl ReviewerClient{
    pub fn new(api_key: String, model: String) -> Self{
        Self {
            client: Client::new(),
            api_key,
            model
        }
    }
    async fn looks_like_topic_change(&self, context_buffer: &str, new_segment: &str) -> anyhow::Result<String>{
        let prompt = format!("Given this context:\n \"{}\"\n and this new segment: \n\"{}\"\nDoes the new segment continue the same topic? If not, what is the new topic? Respond as JSON: {{\"topic_change\": <true/false>, \"new_topic\": <string or null>}}", context_buffer, new_segment);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let resp = self.client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(&self.api_key)
        .json(&body)
        .send()
        .await?
        .json::<LlmResponse>()
        .await?;

        let answer = &resp.choices.get(0)
                    .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?
                    .message.content;
        Ok(answer.clone())
    }


    pub async fn analyze_topic(&self, context: &str, concept: &str,) -> anyhow::Result<String> {
        let prompt = format!(
            r#"You are a student learning about the concept of "{concept}" for the first time. Below is an explanation provided by your teacher as part of the Feynman technique:

            ---
            {context}
            ---

            Your job is to carefully analyze this explanation. If everything is clearly and thoroughly explained, reply with "OK" and nothing else.

            If something is missing, unclear, or only partially explained, ask one or more specific follow-up questions to clarify your understanding. Do not ask questions if the explanation is already complete and clear.

            Remember: Be smart and curious, but you have no prior knowledge beyond what is in the explanation."#,
                        concept = concept,
                        context = context
                    );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .json::<LlmResponse>()
            .await?;

        let answer = &resp
            .choices
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?
            .message
            .content;

        Ok(answer.trim().to_string())
    }

}