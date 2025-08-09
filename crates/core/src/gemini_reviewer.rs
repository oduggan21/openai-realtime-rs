use crate::reviewer::Reviewer;
use crate::topic::SubTopic;
use anyhow::Result;
use async_trait::async_trait;

/// A simulated `Reviewer` for Gemini.
///
/// This implementation does not make any real API calls. It provides plausible,
/// hard-coded responses to allow the application to run with the "gemini" provider
/// without needing a real Gemini Reviewer API. This is useful for testing the
/// real-time audio parts of the Gemini integration independently.
pub struct GeminiReviewer;

#[async_trait]
impl Reviewer for GeminiReviewer {
    async fn looks_like_topic_change(
        &self,
        _context_buffer: &str,
        _new_segment: &str,
    ) -> Result<String> {
        // Simulate that the topic never changes.
        Ok(r#"{"topic_change": false}"#.to_string())
    }

    async fn analyze_topic(
        &self,
        _segment: &str,
        detected_subtopics: &[SubTopic],
    ) -> Result<String> {
        // Simulate that all detected subtopics are perfectly explained.
        let results: Vec<String> = detected_subtopics
            .iter()
            .map(|st| {
                format!(
                    r#"{{"subtopic": "{}", "has_definition": true, "has_mechanism": true, "has_example": true, "questions": []}}"#,
                    st.name
                )
            })
            .collect();
        Ok(format!("[{}]", results.join(",")))
    }

    async fn check_answer_satisfies_question(
        &self,
        _segment: &str,
        _question: &str,
    ) -> Result<bool> {
        // Simulate that all answers are satisfactory.
        Ok(true)
    }

    async fn generate_subtopics(&self, topic: &str) -> Result<Vec<String>> {
        // Simulate a fixed list of subtopics.
        Ok(vec![
            format!("Introduction to {}", topic),
            "Core Concepts".to_string(),
            "Practical Applications".to_string(),
        ])
    }

    async fn analyze_last_explained_context(
        &self,
        _segment: &str,
        _main_topic: &str,
        _subtopic_list: &[String],
    ) -> Result<String> {
        // Simulate a generic, encouraging response.
        Ok("That's a great start! Please continue.".to_string())
    }

    async fn analyze_answer(&self, _question: &str, _answer: &str) -> Result<bool> {
        // Simulate that all answers are correct.
        Ok(true)
    }
}
