use std::time::Instant;

use hyper::{Body, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::canonical::{CanonicalAIRequest, Intent, Payload};
use crate::middleware::{auth, rate_limit::IpRateLimiter};
use crate::middleware::observability::Metrics;
use crate::registry::AdapterRegistryState;

#[derive(Debug, Deserialize)]
pub struct RestCallRequest {
    pub agent_id: String,
    pub intent: String,
    pub payload: RestPayload,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RestPayload {
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
    let parsed: RestCallRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            warn!(error=%e, "invalid json");
            metrics.inc_requests("rest", "400");
            return Ok(status(StatusCode::BAD_REQUEST, "invalid json"));
        }
    };

    let intent = match parsed.intent.as_str() {
        "INTENT_CHAT" | "chat" => Intent::IntentChat,
        "INTENT_EMBED" | "embed" => Intent::IntentEmbed,
        "INTENT_TOOL" | "tool" => Intent::IntentTool,
        _ => Intent::IntentUnspecified,
    };
    let payload = match parsed.payload {
        RestPayload::Text { text } => Payload::Text { text },
        RestPayload::Json { json } => Payload::Json { json },
    };

    let canonical = CanonicalAIRequest::new(parsed.agent_id, intent, payload);
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
