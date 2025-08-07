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
use crate::Input;

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

    pub async fn process_segment(
    session: &mut FeynmanSession,
    reviewer: &ReviewerClient,
    segment: String,
    // Add the two senders as arguments to the function
    input_tx: &tokio::sync::mpsc::Sender<Input>,
    ai_speaking_tx: &tokio::sync::broadcast::Sender<bool>,
) {
    match session.state {
        //in the listneing state we check if we have temp context from previous left over and add it to new segment
        FeynmanState::Listening => {
            let combined = if !session.temp_context_buffer.is_empty() {
                let mut temp = session.temp_context_buffer.join(" ");
                temp.push(' ');
                temp.push_str(&segment);
                session.temp_context_buffer.clear();
                temp
            } else {
                segment
            };

            session.state = FeynmanState::Analyzing;

            match Self::process_analyzing(session, reviewer, combined).await {
                Ok(has_questions) => {
                    if has_questions {
                        // Questions are ready, deliver them
                        if let Err(e) = session
                            .deliver_questions_to_ai(
                                input_tx,
                                ai_speaking_tx,
                                reviewer,
                            )
                            .await
                        {
                            tracing::error!("Failed to deliver questions: {:?}", e);
                            session.state = FeynmanState::Listening;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error during analysis: {:?}", e);
                    session.state = FeynmanState::Listening;
                }
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
    fn process_analyzing<'a>(
    session: &'a mut FeynmanSession,
    reviewer: &'a ReviewerClient,
    segment: String,
) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> {
    Box::pin(async move {
        let detected_subtopics = session.subtopic_list.find_mentions(&segment, 70);

        // if the current segment contains no topics it put it into pending segments to process for later
        if detected_subtopics.is_empty() {
            session.pending_segments.push(segment);
            session.pending_no_subtopic_segment = true;

            if let Some(next_segment) = session.in_between_buffer.pop() {
                // Recursive call to process the next buffered segment
                return Self::process_analyzing(session, reviewer, next_segment).await;
            } else {
                // No more segments to process, go back to listening
                session.state = FeynmanState::Listening;
                return Ok(false); // Fix: Added return for this path
            }
        } else {
            // Combine pending segments and the current segment
            let mut combined = session.pending_segments.join(" ");
            session.pending_segments.clear();
            session.pending_no_subtopic_segment = false;
            if !combined.is_empty() {
                combined.push(' ');
            }
            combined.push_str(&segment);

            let detected_subtopics: Vec<SubTopic> =
                detected_subtopics.into_iter().cloned().collect();
            // Analyze the topic for correctness
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

                    // If a topic was completely covered, add it to the covered subtopics
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
                        // If it contains incomplete subtopics, begin this process
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
            // If no questions were generated then we either continue to next segment or go back to listening
            if question_queue.is_empty() {
                // All subtopics complete
                if let Some(next_segment) = session.in_between_buffer.pop() {
                    return Self::process_analyzing(session, reviewer, next_segment).await;
                } else {
                    session.state = FeynmanState::Listening;
                    return Ok(false); // No questions to deliver
                }
            } else {
                // There are incomplete subtopics/questions: move to question delivery phase
                session.state = FeynmanState::DeliveringQuestions;
                session.question_queue = question_queue;
                session.question_subtopics = incomplete_subtopics;

                // Return true to indicate questions are ready
                return Ok(true); // Questions ready to deliver!
            }
        }
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
    tracing::debug!("Analyzing answer: {}", combined_answer);

    // Analyze with GPT-4
    let is_correct = reviewer
        .analyze_answer(&current_question.question, &combined_answer)
        .await?;

    if is_correct {
        tracing::info!("✓ Correct answer for: {}", current_question.question);
        
        // Update subtopic field in incomplete_subtopics
        self.update_subtopic_field(&current_question.subtopic, &current_question.field, true);
        
        // Check if complete - if so, move from incomplete to covered
        if self.is_subtopic_complete(&current_question.subtopic) {
            if let Some(complete_subtopic) =
                self.incomplete_subtopics.remove(&current_question.subtopic)
            {
                self.covered_subtopics
                    .insert(current_question.subtopic.clone(), complete_subtopic);
                tracing::info!("✓ Subtopic complete: {}", current_question.subtopic);
            }
        }
    } else {
        tracing::info!("✗ Incorrect answer for: {}", current_question.question);
    }

    // Clear answer buffer for next question
    self.answer_buffer.clear();
    
    // INCREMENT the question index for next iteration
    self.current_question_idx += 1;
    
    // Go back to delivering questions (the loop will continue)
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
    async fn wait_for_ai_to_finish_speaking(ai_speaking_tx: &tokio::sync::broadcast::Sender<bool>) {
        let mut receiver = ai_speaking_tx.subscribe();
        
        // Wait for AI to start speaking (true), then finish (false)
        let mut has_started = false;
        
        while let Ok(is_speaking) = receiver.recv().await {
            if is_speaking {
                has_started = true;
                tracing::debug!("AI started speaking, waiting for completion...");
            } else if has_started && !is_speaking {
                tracing::debug!("AI finished speaking");
                break;
            }
        }
    }

    pub async fn deliver_questions_to_ai(
    &mut self,  // Make this a method of self so we can call analyze_answer
    input_tx: &tokio::sync::mpsc::Sender<Input>,
    ai_speaking_tx: &tokio::sync::broadcast::Sender<bool>,
    reviewer: &ReviewerClient,  // Add reviewer so we can pass it to analyze_answer
) -> Result<()> {
    if !matches!(self.state, FeynmanState::DeliveringQuestions) {
        return Ok(());
    }

    // Loop through questions one by one
    while self.current_question_idx < self.question_queue.len() {
        // Get the current question
        let current_question = &self.question_queue[self.current_question_idx];
        tracing::debug!("Delivering question {}/{}: {}", 
            self.current_question_idx + 1, 
            self.question_queue.len(), 
            current_question.question
        );

        // Create item with just this one question
        // 1. First, create the specific content.
        let message_item = openai_realtime_types::content::message::MessageItem::builder()
            .with_role(openai_realtime_types::content::message::MessageRole::User)
            .with_input_text(&current_question.question)
            .build();

        // 2. Then, wrap the built MessageItem inside the Item::Message tuple variant.
        let item = openai_realtime_types::Item::Message(message_item);
                        // Send the question to be spoken
        if let Err(e) = input_tx.send(Input::CreateConversationItem(item)).await {
            eprintln!("Failed to send question to AI: {:?}", e);
            return Err(anyhow::anyhow!("Failed to send question"));
        }

        // Wait for AI to finish speaking the question
        Self::wait_for_ai_to_finish_speaking(ai_speaking_tx).await;
        
        // Transition to analyzing answers for this specific question
        self.state = FeynmanState::AnalyzingAnswers;
        self.answer_buffer.clear();
        tracing::debug!("Waiting for answer to question: {}", current_question.question);

        // Analyze the answer for this question
        self.analyze_answer(reviewer).await?;
        
        // analyze_answer sets state back to DeliveringQuestions and increments the index
        // So the loop will continue with the next question
    }

    // All questions have been delivered and answered
    tracing::info!("All questions delivered and answered");

    // Process any segments that came in during question/answer phase
    if !self.in_between_buffer.is_empty() {
        // Combine all buffered segments
        let buffered_segment = self.in_between_buffer.join(" ");
        tracing::debug!("Processing buffered segments: {}", buffered_segment);

        // Get list of subtopic names for the analysis
        let subtopic_names: Vec<String> = self.subtopic_list.subtopics
            .iter()
            .map(|s| s.name.clone())
            .collect();

        // Analyze where the user left off
        let context_message = reviewer
            .analyze_last_explained_context(
                &buffered_segment,
                &subtopic_names,
            )
            .await?;

        tracing::info!("Context message: {}", context_message);

        // Create item for AI to speak the context message
        let message_item = openai_realtime_types::content::message::MessageItem::builder()
        .with_role(openai_realtime_types::content::message::MessageRole::Assistant)
        .with_input_text(&context_message)
        .build();

    // 2. Then, wrap the built MessageItem inside the Item::Message tuple variant.
    let context_item = openai_realtime_types::Item::Message(message_item);

        // Send the context message to be spoken
        if let Err(e) = input_tx.send(Input::CreateConversationItem(context_item)).await {
            eprintln!("Failed to send context message to AI: {:?}", e);
        }

        // Wait for AI to finish speaking the context message
        Self::wait_for_ai_to_finish_speaking(ai_speaking_tx).await;

        // Move buffered segments to temp context for next analysis round
        self.temp_context_buffer = self.in_between_buffer.drain(..).collect();
    }

    // Clear the question queue since we're done with all questions
    self.question_queue.clear();
    self.current_question_idx = 0;

    // Return to listening state
    self.state = FeynmanState::Listening;
    tracing::info!("Returned to Listening state");

    Ok(())
    
   
}


    
}
