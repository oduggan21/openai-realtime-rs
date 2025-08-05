use crate::topic::{SubTopic, SubTopicList};
use std::collections::HashMap;
use crate::reviewer::{ReviewerClient};
use serde_json::Value;

#[derive(Debug)]
pub enum FeynmanState {
    Listening,
    Analyzing,
    DeliveringQuestions,
    AnalyzingAnswers,
}

pub struct FeynmanSession {
    pub state: FeynmanState,
    pub in_between_buffer: Vec<String>,    // Segments that come in during analyzing/question delivery
    pub answer_buffer: Vec<String>,        // Segments that are answers to questions
    pub temp_context_buffer: Vec<String>,  // Context for "where you left off"
    pub question_queue: Vec<String>,       // Questions to deliver
    pub current_question_idx: usize,
    pub pending_segments: Vec<String>,     // Segments without subtopic (for later combination)
    pub pending_no_subtopic_segment: bool, // True if waiting for a subtopic segment
    pub subtopic_list: SubTopicList,
    pub covered_subtopics: HashMap<String, SubTopic>, // Subtopics covered so far
    pub question_subtopics: Vec<String>,   // Subtopics currently being questioned
    pub incomplete_subtopics: Vec<String>, // Subtopics still missing fields
}

use std::future::Future;
use std::pin::Pin;

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
            incomplete_subtopics: vec![],
        }
    }

    //async function that takes a segment and the current session state and decides what to do from there
    async fn process_segment(session: &mut FeynmanSession, reviewer: &ReviewerClient, segment: String,) {
    match session.state {
        //in the listneing state we check if we have temp context from previous left over and add it to new segment
        FeynmanState::Listening => {
            // If temp_context_buffer is not empty, combine it with segment
            let mut combined = if !session.temp_context_buffer.is_empty() {
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
            Self::process_analyzing(session, reviewer, combined).await;
        }
        FeynmanState::Analyzing => {
            //if we are in an analyzing state and a segment comes in just buffer it
            // While analyzing, buffer incoming segments
            session.in_between_buffer.push(segment);
        }
        // ...other states to be implemented later...
        _ => {}
        }
    }

fn process_analyzing<'a>(
    session: &'a mut FeynmanSession,
    reviewer: &'a ReviewerClient,
    segment: String,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        let detected_subtopics = session.subtopic_list.find_mentions(&segment, 70);

        if detected_subtopics.is_empty() {
            session.pending_segments.push(segment);
            session.pending_no_subtopic_segment = true;
            if let Some(next_segment) = session.in_between_buffer.pop() {
                Self::process_analyzing(session, reviewer, next_segment).await;
            } else {
                session.state = FeynmanState::Listening;
            }
        } else {
            let mut combined = session.pending_segments.join(" ");
            session.pending_segments.clear();
            session.pending_no_subtopic_segment = false;
            if !combined.is_empty() {
                combined.push(' ');
            }
            combined.push_str(&segment);

            let detected_subtopics: Vec<SubTopic> = detected_subtopics.into_iter().cloned().collect();
            let analysis_json = reviewer.analyze_topic(&combined, &detected_subtopics).await.unwrap();

            // Parse LLM output
            let analysis: Value = serde_json::from_str(&analysis_json).expect("Valid JSON");
            let mut all_questions = Vec::new();
            let mut incomplete_subtopics = Vec::new();

            if let Some(array) = analysis.as_array() {
                for subtopic_result in array {
                    let name = subtopic_result["subtopic"].as_str().unwrap_or_default().to_string();
                    let has_def = subtopic_result["has_definition"].as_bool().unwrap_or(false);
                    let has_mech = subtopic_result["has_mechanism"].as_bool().unwrap_or(false);
                    let has_ex = subtopic_result["has_example"].as_bool().unwrap_or(false);
                    let questions: Vec<String> = subtopic_result["questions"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|q| q.as_str().map(|s| s.to_string()))
                        .collect();

                    // Update session.covered_subtopics if complete
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
                        incomplete_subtopics.push(name.clone());
                        all_questions.extend(questions);
                    }
                }
            }

            if all_questions.is_empty() {
                // All subtopics are complete
                if let Some(next_segment) = session.in_between_buffer.pop() {
                    Self::process_analyzing(session, reviewer, next_segment).await;
                } else {
                    session.state = FeynmanState::Listening;
                }
            } else {
                // There are incomplete subtopics/questions: move to question delivery phase
                session.state = FeynmanState::DeliveringQuestions;
                session.question_queue = all_questions;
                session.question_subtopics = incomplete_subtopics;
                // Do NOT process more segments here.
            }
        }
    })
}
    
}