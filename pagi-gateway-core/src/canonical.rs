use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalAIRequest {
    pub request_id: Uuid,
    pub agent_id: String,
    pub intent: Intent,
    #[serde(default)]
    pub constraints: std::collections::BTreeMap<String, String>,
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Intent {
    IntentUnspecified,
    IntentChat,
    IntentEmbed,
    IntentTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    Text { text: String },
    Json { json: serde_json::Value },
}

impl CanonicalAIRequest {
    pub fn new(agent_id: String, intent: Intent, payload: Payload) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            agent_id,
            intent,
            constraints: Default::default(),
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let req = CanonicalAIRequest::new(
            "a".to_string(),
            Intent::IntentChat,
            Payload::Text { text: "hello".to_string() },
        );
        let j = serde_json::to_string(&req).unwrap();
        let back: CanonicalAIRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(back.agent_id, "a");
    }
}

