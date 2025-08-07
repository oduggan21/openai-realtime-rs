use crate::topic::SubTopic;
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
    pub status: String,         // "ok" | "ask" | "clarify_term"
    pub questions: Vec<String>, // 0..=2
}

impl ReviewerClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub async fn looks_like_topic_change(
        &self,
        context_buffer: &str,
        new_segment: &str,
    ) -> anyhow::Result<String> {
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
        Ok(answer.clone())
    }

    pub async fn analyze_topic(
        &self,
        segment: &str,
        detected_subtopics: &[SubTopic],
    ) -> anyhow::Result<String> {
        let subtopic_names = detected_subtopics
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let prompt = format!(
            r#"
        You are a smart beginner in a Feynman-technique session. Analyze the following teacher segment for coverage of the subtopics: [{subtopic_names}].

        For EACH subtopic, answer:
        - Does the segment provide a clear definition for it? (true/false)
        - Does it explain its mechanism or how it works? (true/false)
        - Does it provide a concrete example? (true/false)

        If a field is missing, write a short clarifying question for that field, and indicate which field it corresponds to. Output questions as objects: {{"field": "<field_name>", "question": "<question_text>"}}

        Output STRICT JSON array of objects (one per subtopic):
        [
        {{
            "subtopic": "<name>",
            "has_definition": <true|false>,
            "has_mechanism": <true|false>,
            "has_example": <true|false>,
            "questions": [{{"field": "<field_name>", "question": "<question_text>"}}, ...]
        }},
        ...
        ]

        Teacher segment:
        ---
        {segment}
        ---
    "#
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
        let json_val: serde_json::Value = serde_json::from_str(answer)
            .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {e}"))?;

        // If the output is an object, wrap it in an array.
        let normalized = if json_val.is_array() {
            answer.trim().to_string()
        } else if json_val.is_object() {
            format!("[{}]", answer.trim())
        } else {
            // Unexpected format
            return Err(anyhow::anyhow!(
                "LLM output is not an object or array: {}",
                answer
            ));
        };

        Ok(normalized)
    }
    pub async fn check_answer_satisfies_question(
        &self,
        segment: &str,
        question: &str,
    ) -> anyhow::Result<bool> {
        let prompt = format!(
            r#"Given the following teacher answer segment:
            ---
            {segment}
            ---
            and the question:
            "{question}"

            Does the answer segment satisfactorily answer the question? Respond STRICTLY as a JSON object:
            {{"satisfies": true|false }}

            Do NOT add any explanation, just the JSON."#
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.0 // be as deterministic as possible
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

        // Parse the JSON, expecting {"satisfies": true/false}
        let result: serde_json::Value = serde_json::from_str(answer)?;
        let satisfies = result
            .get("satisfies")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Invalid LLM answer format: {}", answer))?;

        Ok(satisfies)
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

    pub async fn analyze_last_explained_context(
        &self,
        segment: &str,
        subtopic_list: &[String],
    ) -> anyhow::Result<String> {
        // If segment is blank, nudge user to continue
        if segment.trim().is_empty() {
            return Ok(
                "You can continue explaining any part of the topic you'd like. Please keep going!"
                    .to_string(),
            );
        }

        let subtopics = subtopic_list.join(", ");
        let prompt = format!(
            r#"
    You are a Feynman session assistant.
    Given the teacher's latest segment:
    ---
    {segment}
    ---
    and these subtopics: [{subtopics}]

    Identify (in one short sentence) what subtopic or concept the teacher was last explaining, using ONLY the segment and subtopics.

    Respond ONLY as a message to the teacher in this format:
    "You last left off on [subtopic or context]. Please keep telling me more about it."

    Do NOT add any explanation, only output the message.
    "#
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "response_format": { "type": "text" }, // Text: not JSON, just message.
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

    pub async fn analyze_answer(&self, question: &str, answer: &str) -> anyhow::Result<bool> {
        let prompt = format!(
            r#"You are evaluating a student's answer in a Feynman teaching session.

Question: "{question}"

Student's Answer: "{answer}"

Is this answer correct and sufficiently complete for the question asked? 
- The answer should demonstrate understanding of the concept
- It doesn't need to be perfect, but should show the student grasps the main idea
- Consider if a beginner would understand the concept from this explanation

Respond STRICTLY as JSON:
{{"correct": true|false}}

Do NOT add any explanation, just the JSON."#
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.1 // Low temperature for consistent evaluation
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

        // Parse the JSON, expecting {"correct": true/false}
        let result: serde_json::Value = serde_json::from_str(answer)?;
        let is_correct = result
            .get("correct")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Invalid LLM answer format: {}", answer))?;

        Ok(is_correct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topic::SubTopic;
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

    #[tokio::test]
    async fn test_analyze_topic_basic() {
        dotenvy::dotenv_override().ok();
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        let model = "gpt-4o".to_string();
        let reviewer = ReviewerClient::new(api_key, model);

        // Prepare a simple segment and subtopics
        let segment =
            "TCP/IP is a set of networking protocols. For example, the Internet uses TCP/IP.";
        let subtopics = vec![
            SubTopic::new("TCP/IP".to_string()),
            SubTopic::new("Internet".to_string()),
        ];

        let result = reviewer.analyze_topic(segment, &subtopics).await;

        match result {
            Ok(json) => {
                println!("Analysis JSON: {}", json);
                let parsed: serde_json::Value =
                    serde_json::from_str(&json).expect("Should return valid JSON");

                // Accept both array and single object output, wrap as array if necessary
                let arr = if parsed.is_array() {
                    parsed.as_array().unwrap().clone()
                } else if parsed.is_object() {
                    vec![parsed]
                } else {
                    panic!("Output should be a JSON array or object, got {:?}", parsed);
                };

                // Check each subtopic is present
                let subtopic_names = ["TCP/IP", "Internet"];
                for subtopic_name in subtopic_names.iter() {
                    assert!(
                        arr.iter()
                            .any(|obj| obj.get("subtopic").unwrap() == subtopic_name),
                        "Subtopic '{}' should be in result",
                        subtopic_name
                    );
                }

                // Check that questions are structured as objects with 'field' and 'question'
                for obj in arr.iter() {
                    if let Some(questions) = obj.get("questions") {
                        if let Some(qarr) = questions.as_array() {
                            for q in qarr {
                                // Accept old format (just text) OR new format (object with field/question)
                                if q.is_object() {
                                    assert!(
                                        q.get("field").is_some(),
                                        "Question object should have 'field'"
                                    );
                                    assert!(
                                        q.get("question").is_some(),
                                        "Question object should have 'question'"
                                    );
                                } else if q.is_string() {
                                    // Accept legacy string questions for compatibility
                                } else {
                                    panic!("Question should be object or string");
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                panic!("analyze_topic failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_analyze_answer() {
        dotenvy::dotenv_override().ok();
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        let model = "gpt-4o".to_string();
        let reviewer = ReviewerClient::new(api_key, model);

        // Test case 1: Correct and complete answer
        let question = "What is TCP/IP?";
        let good_answer = "TCP/IP is a suite of communication protocols used to interconnect network devices on the internet. It stands for Transmission Control Protocol/Internet Protocol and provides end-to-end communications that specify how data should be packetized, addressed, transmitted, routed and received.";

        let result = reviewer.analyze_answer(question, good_answer).await;
        match result {
            Ok(is_correct) => {
                println!("Good answer evaluation: {}", is_correct);
                assert!(
                    is_correct,
                    "A comprehensive answer should be marked as correct"
                );
            }
            Err(e) => {
                panic!("analyze_answer failed for good answer: {:?}", e);
            }
        }

        // Test case 2: Incorrect answer
        let wrong_answer = "TCP/IP is a type of computer hardware used for storage.";

        let result = reviewer.analyze_answer(question, wrong_answer).await;
        match result {
            Ok(is_correct) => {
                println!("Wrong answer evaluation: {}", is_correct);
                assert!(!is_correct, "An incorrect answer should be marked as false");
            }
            Err(e) => {
                panic!("analyze_answer failed for wrong answer: {:?}", e);
            }
        }

        // Test case 3: Partially correct but incomplete answer
        let incomplete_answer = "TCP/IP is something related to networking.";

        let result = reviewer.analyze_answer(question, incomplete_answer).await;
        match result {
            Ok(is_correct) => {
                println!("Incomplete answer evaluation: {}", is_correct);
                // This might be false since it lacks detail - adjust assertion based on your requirements
                assert!(
                    !is_correct,
                    "An incomplete answer should typically be marked as false"
                );
            }
            Err(e) => {
                panic!("analyze_answer failed for incomplete answer: {:?}", e);
            }
        }

        // Test case 4: Test with a more complex question and answer
        let complex_question = "Explain how memory management works in operating systems";
        let complex_answer = "Memory management in operating systems involves allocating and deallocating memory space to programs. It uses techniques like paging and segmentation to organize memory, virtual memory to extend available RAM using disk space, and implements protection mechanisms to prevent programs from accessing each other's memory spaces.";

        let result = reviewer
            .analyze_answer(complex_question, complex_answer)
            .await;
        match result {
            Ok(is_correct) => {
                println!("Complex answer evaluation: {}", is_correct);
                assert!(
                    is_correct,
                    "A good explanation of memory management should be marked as correct"
                );
            }
            Err(e) => {
                panic!("analyze_answer failed for complex answer: {:?}", e);
            }
        }

        // Test case 5: Empty answer (edge case)
        let empty_answer = "";

        let result = reviewer.analyze_answer(question, empty_answer).await;
        match result {
            Ok(is_correct) => {
                println!("Empty answer evaluation: {}", is_correct);
                assert!(!is_correct, "An empty answer should be marked as false");
            }
            Err(e) => {
                panic!("analyze_answer failed for empty answer: {:?}", e);
            }
        }
    }
}
