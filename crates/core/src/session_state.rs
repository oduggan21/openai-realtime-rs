use crate::{
    reviewer::ReviewerClient,
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

#[derive(Debug)]
pub enum FeynmanState {
    Listening,
    Analyzing,
    DeliveringQuestions,
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

    //async function that takes a segment and the current session state and decides what to do from there
    pub async fn process_segment(
        session: &mut FeynmanSession,
        reviewer: &ReviewerClient,
        segment: String,
    ) {
        match session.state {
            //in the listneing state we check if we have temp context from previous left over and add it to new segment
            FeynmanState::Listening => {
                // If temp_context_buffer is not empty, combine it with segment
                let combined = if !session.temp_context_buffer.is_empty() {
                    //get the temp buffer join all segments, add a space between end of buffer and next segment, and clear the temp buffer
                    let mut temp = session.temp_context_buffer.join(" ");
                    temp.push(' ');
                    temp.push_str(&segment);
                    session.temp_context_buffer.clear();
                    temp
                } else {
                    //if completely new segment just stick with it
                    segment
                };
                // Move to analyzing state
                // switch to the analyzing state and process the current segment with our session, reviewer and segment
                session.state = FeynmanState::Analyzing;
                if let Err(e) = Self::process_analyzing(session, reviewer, combined).await {
                    tracing::error!(
                        "Error during analysis: {:?}. Resetting to Listening state.",
                        e
                    );
                    session.state = FeynmanState::Listening;
                }
            }
            FeynmanState::Analyzing => {
                //if we are in an analyzing state and a segment comes in just buffer it
                // While analyzing, buffer incoming segments
                session.in_between_buffer.push(segment);
            }
            FeynmanState::DeliveringQuestions => session.in_between_buffer.push(segment),
            FeynmanState::AnalyzingAnswers => {
                session.answer_buffer.push(segment);
                session.answer_notify.notify_one()
            }
        }
    }

    // This function returns a Pinned Future to allow for async recursion.
    // The internal helper function `_process_analyzing_inner` contains the actual logic.
    fn process_analyzing<'a>(
        session: &'a mut FeynmanSession,
        reviewer: &'a ReviewerClient,
        segment: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let detected_subtopics = session.subtopic_list.find_mentions(&segment, 70);

            //if the current segment contains no topics it put it into pending segments to process for later
            if detected_subtopics.is_empty() {
                session.pending_segments.push(segment);
                session.pending_no_subtopic_segment = true;
                if let Some(next_segment) = session.in_between_buffer.pop() {
                    // Recursive call is safe here because of the Box::pin indirection.
                    Self::process_analyzing(session, reviewer, next_segment).await?;
                } else {
                    session.state = FeynmanState::Listening;
                }
            } else {
                //combine pending segments and current segment
                let mut combined = session.pending_segments.join(" ");
                session.pending_segments.clear();
                session.pending_no_subtopic_segment = false;
                if !combined.is_empty() {
                    combined.push(' ');
                }
                combined.push_str(&segment);

                let detected_subtopics: Vec<SubTopic> =
                    detected_subtopics.into_iter().cloned().collect();
                //analyze the topic for correctness
                let analysis_json = reviewer
                    .analyze_topic(&combined, &detected_subtopics)
                    .await
                    .context("Reviewer failed to analyze the topic segment")?;

                // Parse LLM output
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

                        //if a topic was completely covered add it to the covered subtopics
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
                            //if it contains incomplete subtopics begin this process
                            session.add_to_incomplete_subtopics(
                                name.clone(),
                                has_def,
                                has_mech,
                                has_ex,
                            );
                            // Now parse questions as objects, not strings!
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
                //if no questions were generated then we either continue to next segment or go back to listenig
                if question_queue.is_empty() {
                    // All subtopics complete
                    if let Some(next_segment) = session.in_between_buffer.pop() {
                        Self::process_analyzing(session, reviewer, next_segment).await?;
                    } else {
                        session.state = FeynmanState::Listening;
                    }
                } else {
                    // There are incomplete subtopics/questions: move to question delivery phase
                    session.state = FeynmanState::DeliveringQuestions;
                    //we have the questions and their subtopics
                    session.question_queue = question_queue;
                    session.question_subtopics = incomplete_subtopics;
                    // Do NOT process more segments here.
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

    pub async fn analyze_answer(&mut self, reviewer: &ReviewerClient) -> Result<()> {
        // Get current question
        let current_question = if self.current_question_idx < self.question_queue.len() {
            self.question_queue[self.current_question_idx].clone()
        } else {
            return Err(anyhow::anyhow!("No current question to analyze answer for"));
        };

        // Wait for answer buffer to have content
        loop {
            if !self.answer_buffer.is_empty() {
                break;
            }
            self.answer_notify.notified().await;
        }

        // Combine answer segments
        let combined_answer = self.answer_buffer.join(" ");

        // Analyze with GPT-4
        let is_correct = reviewer
            .analyze_answer(&current_question.question, &combined_answer)
            .await?;

        if is_correct {
            // Update subtopic field in incomplete_subtopics
            self.update_subtopic_field(&current_question.subtopic, &current_question.field, true);

            // Check if complete - if so, move from incomplete to covered
            if self.is_subtopic_complete(&current_question.subtopic) {
                if let Some(complete_subtopic) =
                    self.incomplete_subtopics.remove(&current_question.subtopic)
                {
                    self.covered_subtopics
                        .insert(current_question.subtopic.clone(), complete_subtopic);
                }
            }
        }

        // Clear answer buffer and go back to delivering questions
        self.answer_buffer.clear();
        self.state = FeynmanState::DeliveringQuestions;

        Ok(())
    }

    // ADD THESE HELPER FUNCTIONS:
    fn update_subtopic_field(&mut self, subtopic_name: &str, field: &str, value: bool) {
        // Update in incomplete_subtopics if it exists, otherwise create entry
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

    fn is_subtopic_complete(&self, subtopic_name: &str) -> bool {
        if let Some(subtopic) = self.incomplete_subtopics.get(subtopic_name) {
            subtopic.has_definition && subtopic.has_mechanism && subtopic.has_example
        } else {
            false
        }
    }
}
