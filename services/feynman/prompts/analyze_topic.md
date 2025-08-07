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