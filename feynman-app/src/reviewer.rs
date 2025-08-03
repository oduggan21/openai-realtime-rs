use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LlmResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub content: String,
}

pub struct ReviewerClient {
    client: Client,
    api_key: String,
    model: String,
}

impl ReviewerClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub async fn looks_like_topic_change(&self, context_buffer: &str, new_segment: &str) -> anyhow::Result<String> {
        let prompt = format!(
            "Given this context:\n \"{context_buffer}\"\n and this new segment: \n\"{new_segment}\"\nDoes the new segment continue the same concept within the topic, when I ask if it continues the same concept I mean someone could be teaching you about football and specifically talking about touchdowns where they talk about how touchdowns are scored, that fits within the concept, but if they start talking about field goals than that is a new concept and you want to say its a new concept? If not, what is the new concept? Respond as JSON: {{\"topic_change\": <true/false>, \"new_topic\": <string or null>}}"
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

    pub async fn analyze_topic(&self, context: &str, concept: &str) -> anyhow::Result<String> {
        let prompt = format!(
            r#"You are a student learning about the concept of "{concept}" for the first time. Below is an explanation provided by your teacher as part of the Feynman technique:

---
{context}
---

Your job is to carefully analyze this explanation. If everything is clearly and thoroughly explained, reply with "OK" and nothing else.

If something is missing, unclear, or only partially explained, ask one or more specific follow-up questions to clarify your understanding. Do not ask questions if the explanation is already complete and clear.

Remember: Be smart and curious, but you have no prior knowledge beyond what is in the explanation."#
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

    pub async fn extract_topic(&self, segment: &str) -> anyhow::Result<String> {
        let prompt = format!(
            "Given this segment:\n\"{segment}\"\nWhat is the main concept or topic being explained? Respond ONLY with the name of the topic as a string."
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
        Ok(answer.trim().to_string())
    }
}