use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Topic {
    pub main_topic: String,
}

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

#[derive(Debug, Clone)]
pub struct SubTopic {
    pub name: String,
    pub has_definition: bool,
    pub has_mechanism: bool,
    pub has_example: bool,
}

impl SubTopic {
    pub fn new(name: String) -> Self {
        Self {
            name,
            has_definition: false,
            has_mechanism: false,
            has_example: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.has_definition && self.has_mechanism && self.has_example
    }
    pub fn score(&self) -> u8 {
        self.has_definition as u8 + self.has_mechanism as u8 + self.has_example as u8
    }
}

pub struct SubTopicList {
    pub subtopics: Vec<SubTopic>,
    matcher: SkimMatcherV2,
}

impl SubTopicList {
    pub fn new(subtopics: Vec<SubTopic>) -> Self {
        Self {
            subtopics,
            matcher: SkimMatcherV2::default(),
        }
    }

    // Returns subtopics whose name matches the segment fuzzily above a threshold
    pub fn find_mentions(&self, segment: &str, threshold: i64) -> Vec<&SubTopic> {
        let segment_lower = segment.to_lowercase();
        self.subtopics
            .iter()
            .filter(|subtopic| {
                let name = subtopic.name.to_lowercase();
                // Fuzzy match (score above threshold, e.g. 70)
                self.matcher.fuzzy_match(&segment_lower, &name).unwrap_or(0) > threshold
            })
            .collect()
    }
}
