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

#[derive(serde::Deserialize, Debug)]
pub struct AnalysisOut {
    pub status: String,                // "ok" | "ask" | "clarify_term"
    pub questions: Vec<String>,        // 0..=2
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
                        r#"You are a smart beginner in a Feynman-technique session learning "{concept}".
                    Act ONLY as the student. Use ONLY the teacher text. Do NOT teach or speculate.

                    Goal: help the teacher find gaps that would block correct use/explanation.

                    Checklist (mentally verify from the text; do not output this list):
                    1) Definition/scope clear?
                    2) Purpose/goal clear?
                    3) Inputs/assumptions/prerequisites stated?
                    4) Decision criteria (when to choose X vs Y) present?
                    5) Core steps/mechanics described?
                    6) Exceptions/edge cases covered?
                    7) Failure outcomes and what happens then?
                    8) Terms/symbols/numbers defined in-context?

                    Question rules:
                    - Ask ≤2 questions targeting the BIGGEST blockers from the checklist.
                    - Make them concrete and actionable (decisions, edge cases, preconditions, failure outcomes).
                    - Avoid trivia, history, “why is constant = X”, or design speculation.
                    - If any term/symbol/number is undefined/unclear, FIRST ask ONE clarification quoting it exactly and STOP. Do NOT guess meaning.

                    Output (STRICT):
                    - JSON ONLY (no prose, no markdown) exactly matching:
                    {{ "status": "ok" | "ask" | "clarify_term", "questions": ["..."] }}
                    - If "ok": questions = [].
                    - If "clarify_term": exactly one clarification question.
                    - If "ask": one or two practical questions, each ≤ 18 words.

                    Teacher text:
                    ---
                    {context}
                    ---"#,
                        concept = concept,
                        context = context
                    );


        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.2
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
    pub async fn generate_subtopics(&self, topic: &str) -> anyhow::Result<Vec<String>> {
    let prompt = format!(
        "List all the key subtopics and concepts someone should cover to thoroughly teach the topic \"{topic}\" to a beginner. \
        Respond ONLY as a numbered list of subtopic names (no explanations)."
    );
    let body = serde_json::json!({
        "model": self.model,
        "messages": [
            { "role": "user", "content": prompt }
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

    // Parse numbered list, extract subtopic names
    let subtopics: Vec<String> = answer
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if let Some(idx) = line.find('.') {
                Some(line[idx + 1..].trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .collect();

    Ok(subtopics)
}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // This test will run only if you have a valid API key in your environment
    #[tokio::test]
    async fn test_generate_subtopics_for_os() {
        dotenvy::dotenv_override().ok(); 
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        let model = "gpt-4o".to_string();
        let reviewer = ReviewerClient::new(api_key, model);

        // Try generating subtopics for "Operating Systems"
        let topic = "Operating Systems";
        let result = reviewer.generate_subtopics(topic).await;

        match result {
            Ok(subtopics) => {
                println!("Subtopics: {:?}", subtopics);
                // You can also assert something about the output, like minimum length
                assert!(subtopics.len() > 3, "Should return at least 3 subtopics");
            }
            Err(e) => {
                panic!("generate_subtopics failed: {:?}", e);
            }
        }
    }
}