use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Normalized role used across provider formats (OpenAI/Anthropic/etc.).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A multimodal content part inside a message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    Image { url: String },
    Audio { url: String },
    File { url: String, mime_type: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message {
    pub role: MessageRole,
    #[serde(default)]
    pub content: Vec<ContentPart>,

    /// Optional name (tool/function name, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional tool_call_id to associate tool results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Tool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for tool parameters (stringified schema or structured JSON).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters_json_schema: Option<serde_json::Value>,

    #[serde(default)]
    pub strict: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GenerationConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    #[serde(default)]
    pub stop_sequences: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// e.g. "low" | "medium" | "high" (provider-specific; normalized as a string)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    #[serde(default)]
    pub stream: bool,
}

impl Default for GenerationConstraints {
    fn default() -> Self {
        Self {
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: vec![],
            presence_penalty: None,
            frequency_penalty: None,
            reasoning_effort: None,
            stream: false,
        }
    }
}

/// Canonical request used by the Rust core to avoid N² protocol/adaptor translation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalAIRequest {
    pub request_id: Uuid,

    /// Agent identity for agentic workflows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,

    /// Conversation/session identifier for persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    #[serde(default)]
    pub messages: Vec<Message>,

    #[serde(default)]
    pub tools: Vec<Tool>,

    /// "auto" | "required" | "none" | <tool_name>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,

    #[serde(default)]
    pub constraints: GenerationConstraints,

    /// Hint for routing (e.g. "gpt-5", "claude-sonnet-4").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<String>,

    /// Flexible metadata for authz/user IDs, cache keys, tracing tags, etc.
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Structured output schema (JSON Schema) for non-tool responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
}

impl CanonicalAIRequest {
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4(),
            agent_id: None,
            session_id: None,
            messages: vec![],
            tools: vec![],
            tool_choice: None,
            constraints: GenerationConstraints::default(),
            preferred_model: None,
            metadata: HashMap::new(),
            response_format: None,
        }
    }

    pub fn chat_text(agent_id: Option<String>, text: String) -> Self {
        let mut req = Self::new();
        req.agent_id = agent_id;
        req.messages.push(Message {
            role: MessageRole::User,
            content: vec![ContentPart::Text { text }],
            name: None,
            tool_call_id: None,
        });
        req
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let req = CanonicalAIRequest::chat_text(Some("a".to_string()), "hello".to_string());
        let j = serde_json::to_string(&req).unwrap();
        let back: CanonicalAIRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(back.agent_id.as_deref(), Some("a"));
        assert_eq!(back.messages.len(), 1);
    }
}

