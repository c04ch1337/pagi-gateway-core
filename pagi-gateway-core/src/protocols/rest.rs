use std::collections::HashMap;
use std::time::Instant;

use hyper::{Body, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::canonical::{CanonicalAIRequest, ContentPart, GenerationConstraints, Message, MessageRole, Tool};
use crate::middleware::{auth, rate_limit::IpRateLimiter};
use crate::middleware::observability::Metrics;
use crate::registry::AdapterRegistryState;

/// Accept both the legacy MVP shape and the newer canonical-ish shape.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RestIngressRequest {
    V0(LegacyV0Request),
    V1(LegacyV1Request),
    V2(CanonicalIngressRequest),
}

/// Canonical-ish request without requiring client to provide request_id.
#[derive(Debug, Deserialize)]
pub struct CanonicalIngressRequest {
    #[serde(default)]
    pub request_id: Option<Uuid>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub messages: Vec<RestMessageInput>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub tool_choice: Option<String>,
    #[serde(default)]
    pub constraints: Option<GenerationConstraints>,
    #[serde(default)]
    pub preferred_model: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub response_format: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RestMessageInput {
    pub role: MessageRole,
    #[serde(default)]
    pub content: Vec<RestContentPartInput>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RestContentPartInput {
    // Canonical wire form: {"type":"text","text":"..."}
    Canonical(ContentPart),
    // Shorthand: {"text":"..."}
    Text { text: String },
    // Shorthand: {"image": {"url":"..."}}
    Image { image: UrlOnly },
    Audio { audio: UrlOnly },
    File { file: FileOnly },
}

#[derive(Debug, Deserialize)]
pub struct UrlOnly {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct FileOnly {
    pub url: String,
    pub mime_type: String,
}

#[derive(Debug, Deserialize)]
pub struct LegacyV1Request {
    pub agent_id: String,
    pub intent: String,
    pub payload: LegacyPayload,
}

/// Extra-legacy request shape (single string payload, no explicit intent).
#[derive(Debug, Deserialize)]
pub struct LegacyV0Request {
    pub agent_id: String,
    pub payload: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LegacyPayload {
    Text { text: String },
    Json { json: serde_json::Value },
}

#[derive(Debug, Serialize)]
pub struct RestCallResponse {
    pub request_id: String,
    pub adapter_id: String,
    pub json: String,
}

pub async fn handle_call(
    req: Request<Body>,
    registry: AdapterRegistryState,
    metrics: Metrics,
) -> Result<Response<Body>, hyper::Error> {
    let started = Instant::now();
    let limiter = IpRateLimiter::new_per_second(50);

    if !auth::authorize(&req) {
        metrics.inc_requests("rest", "401");
        return Ok(status(StatusCode::UNAUTHORIZED, "unauthorized"));
    }

    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .to_string();
    if !limiter.check(ip) {
        metrics.inc_requests("rest", "429");
        return Ok(status(StatusCode::TOO_MANY_REQUESTS, "rate limited"));
    }

    let body = hyper::body::to_bytes(req.into_body()).await?;
    let parsed: RestIngressRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            warn!(error=%e, "invalid json");
            metrics.inc_requests("rest", "400");
            return Ok(status(StatusCode::BAD_REQUEST, "invalid json"));
        }
    };

    let canonical = match parsed {
        RestIngressRequest::V2(v) => {
            let mut req = CanonicalAIRequest::new();
            if let Some(id) = v.request_id {
                req.request_id = id;
            }
            req.agent_id = v.agent_id;
            req.session_id = v.session_id;
            req.messages = v
                .messages
                .into_iter()
                .map(|m| Message {
                    role: m.role,
                    content: m
                        .content
                        .into_iter()
                        .map(|p| match p {
                            RestContentPartInput::Canonical(p) => p,
                            RestContentPartInput::Text { text } => ContentPart::Text { text },
                            RestContentPartInput::Image { image } => ContentPart::Image { url: image.url },
                            RestContentPartInput::Audio { audio } => ContentPart::Audio { url: audio.url },
                            RestContentPartInput::File { file } => ContentPart::File { url: file.url, mime_type: file.mime_type },
                        })
                        .collect(),
                    name: m.name,
                    tool_call_id: m.tool_call_id,
                })
                .collect();
            req.tools = v.tools;
            req.tool_choice = v.tool_choice;
            req.constraints = v.constraints.unwrap_or_default();
            req.preferred_model = v.preferred_model;
            req.metadata = v.metadata;
            req.response_format = v.response_format;
            req
        }
        RestIngressRequest::V1(v) => {
            // Legacy request maps to a single user message.
            let mut req = CanonicalAIRequest::chat_text(Some(v.agent_id), match v.payload {
                LegacyPayload::Text { text } => text,
                LegacyPayload::Json { json } => json.to_string(),
            });
            req.metadata.insert("legacy_intent".to_string(), v.intent);
            req
        }
        RestIngressRequest::V0(v) => CanonicalAIRequest::chat_text(Some(v.agent_id), v.payload),
    };

    // Convenience: allow clients to specify a preferred provider without needing to know
    // the internal routing key name.
    let mut canonical = canonical;
    if !canonical.metadata.contains_key("adapter_id") {
        if let Some(p) = canonical.metadata.get("preferred_provider").cloned() {
            canonical.metadata.insert("adapter_id".to_string(), p);
        }
    }

    // If client sent an empty messages list, treat as invalid.
    if canonical.messages.is_empty() {
        metrics.inc_requests("rest", "400");
        return Ok(status(StatusCode::BAD_REQUEST, "messages required"));
    }

    info!(request_id=%canonical.request_id, "canonicalized rest request");

    let resp = match registry.forward(canonical.clone()).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error=%e, "forward failed");
            metrics.inc_requests("rest", "503");
            return Ok(status(StatusCode::SERVICE_UNAVAILABLE, "no adapter available"));
        }
    };

    metrics.inc_requests("rest", "200");
    metrics.observe_latency("rest", started.elapsed().as_secs_f64());

    let out = RestCallResponse { request_id: resp.request_id, adapter_id: resp.adapter_id, json: resp.json };
    Ok(json(StatusCode::OK, &out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_v0() {
        let j = r#"{"agent_id":"a","payload":"hello"}"#;
        let parsed: RestIngressRequest = serde_json::from_str(j).unwrap();
        match parsed {
            RestIngressRequest::V0(v) => assert_eq!(v.payload, "hello"),
            _ => panic!("expected v0"),
        }
    }

    #[test]
    fn parses_content_part_shorthand() {
        let j = r#"{
          "messages": [{
            "role": "user",
            "content": [{"text":"hi"}, {"image": {"url": "https://e/x.png"}}]
          }]
        }"#;
        let parsed: CanonicalIngressRequest = serde_json::from_str(j).unwrap();
        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(parsed.messages[0].content.len(), 2);
    }
}

fn json<T: serde::Serialize>(status: StatusCode, v: &T) -> Response<Body> {
    let body = serde_json::to_vec(v).unwrap();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

fn status(status: StatusCode, msg: &str) -> Response<Body> {
    Response::builder().status(status).body(Body::from(msg.to_string())).unwrap()
}
