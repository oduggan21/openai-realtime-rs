#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    total_tokens: i32,
    input_tokens: i32,
    output_tokens: i32,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    pub(crate) fn update_usage(&mut self, total: i32, input: i32, output: i32) {
        self.total_tokens += total;
        self.input_tokens += input;
        self.output_tokens += output;
    }
    
    fn total_tokens(&self) -> i32 {
        self.total_tokens
    }
    
    fn input_tokens(&self) -> i32 {
        self.input_tokens
    }
    
    fn output_tokens(&self) -> i32 {
        self.output_tokens
    }
}
