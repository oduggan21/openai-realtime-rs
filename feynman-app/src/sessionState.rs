#[derive(Debug)]
pub enum FeynmanState {
    Listening,
    Analyzing,
    DeliveringQuestions,
    AnalyzingAnswers,
}

pub struct FeynmanSession {
    pub state: FeynmanState,
    // Segments currently awaiting analysis,
    // can be a single segment, or in-between context
    pub in_between_buffer: Vec<String>,
    // Answer segments for questions
    pub answer_buffer: Vec<String>,
    // Temp buffer for "where did you leave off" context
    pub temp_context_buffer: Vec<String>,
    // Questions currently being asked
    pub question_queue: Vec<String>,
    // Index of current question
    pub current_question_idx: usize,
}

impl FeynmanSession {
    pub fn new() -> Self {
        Self {
            state: FeynmanState::Listening,
            in_between_buffer: vec![],
            answer_buffer: vec![],
            temp_context_buffer: vec![],
            question_queue: vec![],
            current_question_idx: 0,
        }
    }

    // Call this when a segment arrives
    pub fn handle_segment(&mut self, segment: String) {
        match self.state {
            FeynmanState::Listening => {
                self.in_between_buffer.push(segment);
                self.state = FeynmanState::Analyzing;
                // trigger analysis here
            }
            FeynmanState::Analyzing | FeynmanState::DeliveringQuestions => {
                self.in_between_buffer.push(segment);
            }
            FeynmanState::AnalyzingAnswers => {
                self.answer_buffer.push(segment);
            }
        }
    }

    // Call this when analysis is done
    pub fn handle_analysis_result(&mut self, questions: Vec<String>) {
        if questions.is_empty() {
            // Previous segment was OK, check for more segments to analyze.
            if !self.in_between_buffer.is_empty() {
                // Pop the next segment to analyze (FIFO: remove first)
                let next_segment = self.in_between_buffer.remove(0);
                // Optionally, move OK segments to explanation_buffer if you want a history
                // self.explanation_buffer.push(next_segment.clone());
                // Now analyze next_segment
                // You would trigger analysis here, e.g.:
                // analyze_segment(next_segment)
                // Set state to Analyzing to process the next one
                self.state = FeynmanState::Analyzing;
            } else {
                // No more segments to process, go back to Listening
                self.state = FeynmanState::Listening;
            }
        } else {
            // If questions are generated, move to DeliveringQuestions state
            self.question_queue = questions;
            self.current_question_idx = 0;
            self.state = FeynmanState::DeliveringQuestions;
        }
    }

    // Call after all questions delivered
    pub fn finish_delivering_questions(&mut self) {
        self.state = FeynmanState::AnalyzingAnswers;
    }

    // Call after answer segment is processed
    pub fn handle_answer_result(&mut self, correct: bool) {
        if correct {
            // Remove current question from queue
            if !self.question_queue.is_empty() {
                self.question_queue.remove(self.current_question_idx);
            }
            // Output "That makes sense"
        } else {
            // Offer retry: stay in AnalyzingAnswers, clear buffer for new answer
            self.answer_buffer.clear();
            return;
        }

        // If more questions, process next
        if !self.question_queue.is_empty() {
            self.current_question_idx = 0;
            self.answer_buffer.clear();
        } else {
            // All questions handled, resume flow
            self.finish_questions_and_resume();
        }
    }

    // Call after all questions answered and ready to resume
    pub fn finish_questions_and_resume(&mut self) {
        self.temp_context_buffer = self.in_between_buffer.drain(..).collect();
        // Output context to user: "You were last explaining: {context}"
        self.state = FeynmanState::Listening;
        // Next incoming segment will be appended to temp_context_buffer and analyzed
    }

    // When a new segment arrives after resume
    pub fn handle_resume_segment(&mut self, segment: String) {
        // Combine with temp_context_buffer and analyze
        let mut combined = self.temp_context_buffer.clone();
        combined.push(segment);
        let all = combined.join(" ");
        self.in_between_buffer.clear();
        self.in_between_buffer.push(all);
        self.temp_context_buffer.clear();
        self.state = FeynmanState::Analyzing;
        // trigger analysis again
    }
}