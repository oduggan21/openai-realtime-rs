use crate::{
    Command,
    reviewer::Reviewer,
    topic::{SubTopic, SubTopicList},
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct QuestionForSubtopic {
    pub subtopic: String,
    pub field: String, // "has_definition" | "has_mechanism" | "has_example"
    pub question: String,
}

#[derive(Debug, PartialEq)]
pub enum FeynmanState {
    Listening,
    Analyzing,
    AnalyzingAnswers,
}

pub struct FeynmanSession {
    pub state: FeynmanState,
    pub in_between_buffer: Vec<String>, // Segments that come in during analyzing/question delivery
    pub answer_buffer: Vec<String>,     // Segments that are answers to questions
    pub temp_context_buffer: Vec<String>, // Context for "where you left off"
    pub question_queue: Vec<QuestionForSubtopic>, // Questions to deliver
    pub current_question_idx: usize,
    pub pending_segments: Vec<String>, // Segments without subtopic (for later combination)
    pub pending_no_subtopic_segment: bool, // True if waiting for a subtopic segment
    pub subtopic_list: SubTopicList,
    pub covered_subtopics: HashMap<String, SubTopic>, // Subtopics covered so far
    pub question_subtopics: Vec<String>,              // Subtopics currently being questioned
    pub incomplete_subtopics: HashMap<String, SubTopic>,
    pub answer_notify: Arc<Notify>,
}

impl FeynmanSession {
    pub fn new(subtopic_list: SubTopicList) -> Self {
        Self {
            state: FeynmanState::Listening,
            in_between_buffer: vec![],
            answer_buffer: vec![],
            temp_context_buffer: vec![],
            question_queue: vec![],
            current_question_idx: 0,
            pending_segments: vec![],
            pending_no_subtopic_segment: false,
            subtopic_list,
            covered_subtopics: HashMap::new(),
            question_subtopics: vec![],
            incomplete_subtopics: HashMap::new(),
            answer_notify: Arc::new(Notify::new()),
        }
    }

    // This function is now generic over any type `R` that implements the `Reviewer` trait.
    // The `Send + Sync` bounds are required because the `reviewer` is used in an `async`
    // context (`process_analyzing`) which may be run on a different thread.
    pub async fn process_segment<R: Reviewer + Send + Sync>(
        session: &mut FeynmanSession,
        reviewer: &R,
        segment: String,
        command_tx: tokio::sync::mpsc::Sender<Command>,
    ) {
        match session.state {
            // In the listening state, we check if we have temp context from a previous leftover segment and add it to the new segment.
            FeynmanState::Listening => {
                // If temp_context_buffer is not empty, combine it with the new segment.
                let combined = if !session.temp_context_buffer.is_empty() {
                    // Get the temp buffer, join all segments, add a space, and append the new segment.
                    let mut temp = session.temp_context_buffer.join(" ");
                    temp.push(' ');
                    temp.push_str(&segment);
                    session.temp_context_buffer.clear();
                    temp
                } else {
                    // If there's no temp context, just use the new segment.
                    segment
                };
                // Move to the analyzing state and process the combined segment.
                session.state = FeynmanState::Analyzing;
                if let Err(e) =
                    Self::process_analyzing(session, reviewer, combined, command_tx).await
                {
                    tracing::error!(
                        "Error during analysis: {:?}. Resetting to Listening state.",
                        e
                    );
                    session.state = FeynmanState::Listening;
                }
            }
            FeynmanState::Analyzing => {
                // If we are in an analyzing state and a new segment comes in, just buffer it for later.
                session.in_between_buffer.push(segment);
            }
            FeynmanState::AnalyzingAnswers => {
                session.answer_buffer.push(segment);
                session.answer_notify.notify_one()
            }
        }
    }

    // This function returns a Pinned Future to allow for async recursion.
    // It's generic over `R: Reviewer` and also requires `Send + Sync` because the
    // returned Future might be sent across threads (e.g., in a `tokio::spawn`).
    // The `'a` lifetime ensures that the reference to the reviewer lives as long as the Future.
    fn process_analyzing<'a, R: Reviewer + Send + Sync>(
        session: &'a mut FeynmanSession,
        reviewer: &'a R,
        segment: String,
        command_tx: tokio::sync::mpsc::Sender<Command>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    where
        R: 'a,
    {
        Box::pin(async move {
            let detected_subtopics = session.subtopic_list.find_mentions(&segment, 70);

            // If the current segment contains no topics, put it into pending segments to process later.
            if detected_subtopics.is_empty() {
                session.pending_segments.push(segment);
                session.pending_no_subtopic_segment = true;
                if let Some(next_segment) = session.in_between_buffer.pop() {
                    // Recursive call is safe here because of the Box::pin indirection.
                    Self::process_analyzing(session, reviewer, next_segment, command_tx).await?;
                } else {
                    session.state = FeynmanState::Listening;
                }
            } else {
                // Combine pending segments with the current segment.
                let mut combined = session.pending_segments.join(" ");
                session.pending_segments.clear();
                session.pending_no_subtopic_segment = false;
                if !combined.is_empty() {
                    combined.push(' ');
                }
                combined.push_str(&segment);

                let detected_subtopics: Vec<SubTopic> =
                    detected_subtopics.into_iter().cloned().collect();
                // Analyze the topic for correctness using the reviewer.
                let analysis_json = reviewer
                    .analyze_topic(&combined, &detected_subtopics)
                    .await
                    .context("Reviewer failed to analyze the topic segment")?;

                // Parse the LLM's JSON output.
                let analysis: Value = serde_json::from_str(&analysis_json)
                    .context("Failed to parse JSON from reviewer's analysis")?;

                let mut question_queue: Vec<QuestionForSubtopic> = vec![];
                let incomplete_subtopics = Vec::new();

                if let Some(array) = analysis.as_array() {
                    for subtopic_result in array {
                        let name = subtopic_result["subtopic"]
                            .as_str()
                            .context("Subtopic result in JSON is missing a 'subtopic' field")?
                            .to_string();
                        let has_def = subtopic_result["has_definition"].as_bool().unwrap_or(false);
                        let has_mech = subtopic_result["has_mechanism"].as_bool().unwrap_or(false);
                        let has_ex = subtopic_result["has_example"].as_bool().unwrap_or(false);

                        // If a topic was completely covered, add it to the covered subtopics.
                        if has_def && has_mech && has_ex {
                            session.covered_subtopics.insert(
                                name.clone(),
                                SubTopic {
                                    name: name.clone(),
                                    has_definition: has_def,
                                    has_mechanism: has_mech,
                                    has_example: has_ex,
                                },
                            );
                        } else {
                            // If there are incomplete subtopics, add them to the list.
                            session.add_to_incomplete_subtopics(
                                name.clone(),
                                has_def,
                                has_mech,
                                has_ex,
                            );
                            // Parse the questions generated by the LLM.
                            if let Some(questions_arr) = subtopic_result["questions"].as_array() {
                                for q in questions_arr {
                                    let field = q
                                        .get("field")
                                        .and_then(|f| f.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let question = q
                                        .get("question")
                                        .and_then(|f| f.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    question_queue.push(QuestionForSubtopic {
                                        subtopic: name.clone(),
                                        field,
                                        question,
                                    });
                                }
                            }
                        }
                    }
                }
                // If no questions were generated, we either continue to the next segment or go back to listening.
                if question_queue.is_empty() {
                    // All subtopics are complete.
                    if let Some(next_segment) = session.in_between_buffer.pop() {
                        Self::process_analyzing(session, reviewer, next_segment, command_tx)
                            .await?;
                    } else {
                        session.state = FeynmanState::Listening;
                    }
                } else {
                    // There are incomplete subtopics/questions.
                    session.question_queue = question_queue;
                    session.question_subtopics = incomplete_subtopics;
                    session.current_question_idx = 0; // Start with the first question.

                    // Get the first question from the now-populated queue.
                    if let Some(first_question) = session.question_queue.get(0) {
                        // Send a command to the runtime to ask the question.
                        command_tx
                            .send(Command::SpeakText(first_question.question.clone()))
                            .await
                            .context("Failed to send SpeakText command")?;

                        // After commanding the runtime to ask, we wait for the answer.
                        session.state = FeynmanState::AnalyzingAnswers;
                    } else {
                        // This case should not be reached if the queue is not empty, but as a safeguard:
                        session.state = FeynmanState::Listening;
                    }
                }
            }
            Ok(())
        })
    }

    fn add_to_incomplete_subtopics(
        &mut self,
        name: String,
        has_def: bool,
        has_mech: bool,
        has_ex: bool,
    ) {
        self.incomplete_subtopics.insert(
            name.clone(),
            SubTopic {
                name,
                has_definition: has_def,
                has_mechanism: has_mech,
                has_example: has_ex,
            },
        );
    }

    pub async fn analyze_answer<R: Reviewer + Send + Sync>(
        &mut self,
        reviewer: &R,
        command_tx: tokio::sync::mpsc::Sender<Command>,
    ) -> Result<()> {
        // Get the current question.
        let current_question = if self.current_question_idx < self.question_queue.len() {
            self.question_queue[self.current_question_idx].clone()
        } else {
            return Err(anyhow::anyhow!("No current question to analyze answer for"));
        };

        // Wait for the answer buffer to have content.
        loop {
            if !self.answer_buffer.is_empty() {
                break;
            }
            self.answer_notify.notified().await;
        }

        // Combine answer segments into a single string.
        let combined_answer = self.answer_buffer.join(" ");

        // Analyze the answer with the reviewer.
        let is_correct = reviewer
            .analyze_answer(&current_question.question, &combined_answer)
            .await?;

        if is_correct {
            // Update the subtopic field in incomplete_subtopics.
            self.update_subtopic_field(&current_question.subtopic, &current_question.field, true);

            // Check if the subtopic is now complete. If so, move it from incomplete to covered.
            if self.is_subtopic_complete(&current_question.subtopic) {
                if let Some(complete_subtopic) =
                    self.incomplete_subtopics.remove(&current_question.subtopic)
                {
                    self.covered_subtopics
                        .insert(current_question.subtopic.clone(), complete_subtopic);
                }
            }
        }

        // Clear the buffer for the next answer.
        self.answer_buffer.clear();
        // Move to the next question.
        self.current_question_idx += 1;

        if self.current_question_idx < self.question_queue.len() {
            // If there is a next question, command the runtime to ask it.
            let next_question = &self.question_queue[self.current_question_idx];
            command_tx
                .send(Command::SpeakText(next_question.question.clone()))
                .await
                .context("Failed to send next SpeakText command")?;
            // The state remains AnalyzingAnswers, as we are now waiting for the next answer.
        } else {
            // All questions for this batch have been answered.
            let final_message =
                "Great, you've answered all the questions for now. Let's continue.".to_string();
            command_tx
                .send(Command::SessionComplete(final_message))
                .await
                .context("Failed to send SessionComplete command")?;

            // Reset state and queues, ready to listen for the next topic explanation.
            self.question_queue.clear();
            self.current_question_idx = 0;
            self.state = FeynmanState::Listening;

            // TODO: In a future iteration, we should handle the `in_between_buffer` here
            // to process segments that arrived while questions were being answered.
        }

        Ok(())
    }

    // Helper function to update a field of a subtopic.
    fn update_subtopic_field(&mut self, subtopic_name: &str, field: &str, value: bool) {
        // Update in incomplete_subtopics if it exists, otherwise create a new entry.
        let subtopic = self
            .incomplete_subtopics
            .entry(subtopic_name.to_string())
            .or_insert_with(|| SubTopic {
                name: subtopic_name.to_string(),
                has_definition: false,
                has_mechanism: false,
                has_example: false,
            });

        match field {
            "has_definition" => subtopic.has_definition = value,
            "has_mechanism" => subtopic.has_mechanism = value,
            "has_example" => subtopic.has_example = value,
            _ => {}
        }
    }

    // Helper function to check if a subtopic is fully covered.
    fn is_subtopic_complete(&self, subtopic_name: &str) -> bool {
        if let Some(subtopic) = self.incomplete_subtopics.get(subtopic_name) {
            subtopic.has_definition && subtopic.has_mechanism && subtopic.has_example
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reviewer::MockReviewer;
    use crate::topic::SubTopic;

    #[tokio::test]
    async fn test_process_segment_generates_questions() {
        // --- 1. Arrange ---
        // Create a mock reviewer. This allows us to control its behavior without making real API calls.
        let mut mock_reviewer = MockReviewer::new();

        // Set up an expectation: when `analyze_topic` is called, it should return a specific JSON string.
        // This simulates the LLM finding an incomplete subtopic and generating a question.
        mock_reviewer
            .expect_analyze_topic()
            .returning(|_segment, _subtopics| {
                let json_response = r#"[
                    {
                        "subtopic": "TCP/IP",
                        "has_definition": false,
                        "has_mechanism": false,
                        "has_example": false,
                        "questions": [
                            {
                                "field": "has_definition",
                                "question": "What is TCP/IP?"
                            }
                        ]
                    }
                ]"#;
                Box::pin(async move { Ok(json_response.to_string()) })
            })
            .once(); // We expect this to be called exactly once.

        // Create a dummy list of subtopics for the session.
        let subtopics = vec![SubTopic::new("TCP/IP".to_string())];
        let subtopic_list = SubTopicList::new(subtopics);

        // Create a new FeynmanSession, starting in the Listening state.
        let mut session = FeynmanSession::new(subtopic_list);
        let segment = "Let's talk about TCP/IP.".to_string();

        // Create a dummy channel for the command. We'll check if a command was sent.
        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(1);

        // --- 2. Act ---
        // Process the segment. This will trigger the call to the (mock) reviewer.
        FeynmanSession::process_segment(&mut session, &mock_reviewer, segment, command_tx).await;

        // --- 3. Assert ---
        // Check that the session state has transitioned correctly to wait for an answer.
        assert_eq!(
            session.state,
            FeynmanState::AnalyzingAnswers,
            "Session should be in AnalyzingAnswers state"
        );

        // Check that a command was actually sent.
        let received_command = command_rx
            .try_recv()
            .expect("A command should have been sent");
        match received_command {
            Command::SpeakText(text) => {
                assert_eq!(text, "What is TCP/IP?");
            }
            _ => panic!("Expected a SpeakText command"),
        }

        // Check that the question queue has been populated with the expected question.
        assert_eq!(
            session.question_queue.len(),
            1,
            "Question queue should have one question"
        );
        let question = &session.question_queue[0];
        assert_eq!(question.subtopic, "TCP/IP");
        assert_eq!(question.field, "has_definition");
        assert_eq!(question.question, "What is TCP/IP?");
    }
}
