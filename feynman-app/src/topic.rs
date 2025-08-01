use serde::Deserialize;

// struct to hold response from topic analyzer
#[derive(Deserialize, Debug)]
pub struct TopicChange {
    pub topic_change: bool,
    pub new_topic: Option<String>,
}

// object to hold current topic and topic segments
#[derive(Debug)]
pub struct TopicBuffer {
    pub topic: String,
    pub segments: Vec<String>,
}

impl TopicBuffer {
    pub fn new(topic: String) -> Self {
        Self {
            topic,
            segments: Vec::new(),
        }
    }

    pub fn add_segment(&mut self, segment: String) {
        self.segments.push(segment);
    }

    pub fn clear(&mut self) {
        self.topic.clear();
        self.segments.clear();
    }
}