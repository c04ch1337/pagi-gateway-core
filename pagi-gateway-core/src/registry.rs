use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::info;

use crate::canonical::{CanonicalAIRequest, ContentPart, MessageRole};
use crate::config::RequestReplayConfig;
use crate::proto::{
    adapter_registry_server::{AdapterRegistry, AdapterRegistryServer},
    adapter_service_client::AdapterServiceClient,
    AdapterCapabilities, AdapterInfo, CanonicalAiRequest, CanonicalAiResponse, ContentPart as ProtoContentPart,
    FilePart as ProtoFilePart, GenerationConstraints as ProtoGenerationConstraints, ImagePart as ProtoImagePart,
    Message as ProtoMessage, Tool as ProtoTool, TextPart as ProtoTextPart, AudioPart as ProtoAudioPart,
    ListAdaptersRequest, ListAdaptersResponse, RegisterAdapterRequest, RegisterAdapterResponse,
};

#[derive(Clone)]
pub struct AdapterRegistryState {
    inner: Arc<Inner>,
}

struct Inner {
    adapters: RwLock<BTreeMap<String, AdapterInfo>>,
    replay: RequestReplayConfig,
}

#[derive(Debug, Clone)]
pub struct ForwardResponse {
    pub request_id: String,
    pub adapter_id: String,
    pub json: String,
}

impl AdapterRegistryState {
    pub fn new(replay: RequestReplayConfig) -> Self {
        Self {
            inner: Arc::new(Inner {
                adapters: RwLock::new(BTreeMap::new()),
                replay,
            }),
        }
    }

    pub async fn forward(&self, req: CanonicalAIRequest) -> anyhow::Result<ForwardResponse> {
        self.maybe_replay(&req).await;

        let adapters = self.inner.adapters.read().await;
        let mut candidates: Vec<(String, AdapterInfo)> = Vec::new();

        if let Some(id) = req.metadata.get("adapter_id") {
            if let Some(info) = adapters.get(id) {
                candidates.push((id.clone(), info.clone()));
            }
        } else {
            // Default routing policy:
            // 1) OpenRouter if registered
            // 2) Ollama failover if registered
            // 3) otherwise first-registered adapter
            if let Some(info) = adapters.get("openrouter") {
                candidates.push(("openrouter".to_string(), info.clone()));
            }
            if let Some(info) = adapters.get("ollama") {
                candidates.push(("ollama".to_string(), info.clone()));
            }
            if let Some((k, v)) = adapters.iter().next() {
                // Avoid duplicates.
                if !candidates.iter().any(|(id, _)| id == k) {
                    candidates.push((k.clone(), v.clone()));
                }
            }
        }

        drop(adapters);

        let proto_req: CanonicalAiRequest = to_proto(req);
        let mut last_err: Option<anyhow::Error> = None;

        for (adapter_id, adapter) in candidates {
            let endpoint = adapter.endpoint.clone();
            let attempt = async {
                let mut client = AdapterServiceClient::connect(endpoint).await?;
                let resp: CanonicalAiResponse = client.process(proto_req.clone()).await?.into_inner();
                Ok::<_, anyhow::Error>(ForwardResponse { request_id: resp.request_id, adapter_id, json: resp.json })
            }
            .await;

            match attempt {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no adapters registered")))
    }

    async fn maybe_replay(&self, req: &CanonicalAIRequest) {
        if !self.inner.replay.enabled {
            return;
        }
        if let Ok(line) = serde_json::to_string(req) {
            if let Ok(mut f) = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.inner.replay.path)
                .await
            {
                use tokio::io::AsyncWriteExt;
                let _ = f.write_all(line.as_bytes()).await;
                let _ = f.write_all(b"\n").await;
            }
        }
    }
}

pub struct AdapterRegistrySvc {
    state: AdapterRegistryState,
}

impl AdapterRegistrySvc {
    pub fn new(state: AdapterRegistryState) -> AdapterRegistryServer<Self> {
        AdapterRegistryServer::new(Self { state })
    }
}

#[tonic::async_trait]
impl AdapterRegistry for AdapterRegistrySvc {
    async fn register(
        &self,
        request: Request<RegisterAdapterRequest>,
    ) -> Result<Response<RegisterAdapterResponse>, Status> {
        let r = request.into_inner();
        if r.adapter_id.is_empty() || r.endpoint.is_empty() {
            return Err(Status::invalid_argument("adapter_id and endpoint required"));
        }
        let caps = r.capabilities.unwrap_or(AdapterCapabilities::default());
        let info = AdapterInfo { adapter_id: r.adapter_id.clone(), endpoint: r.endpoint.clone(), capabilities: Some(caps), version: r.version };
        self.state.inner.adapters.write().await.insert(r.adapter_id.clone(), info);
        info!(adapter_id=%r.adapter_id, endpoint=%r.endpoint, "adapter registered");
        Ok(Response::new(RegisterAdapterResponse { ok: true }))
    }

    async fn list(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self
            .state
            .inner
            .adapters
            .read()
            .await
            .values()
            .cloned()
            .collect();
        Ok(Response::new(ListAdaptersResponse { adapters }))
    }
}

fn to_proto(req: CanonicalAIRequest) -> CanonicalAiRequest {
    let messages = req
        .messages
        .into_iter()
        .map(|m| ProtoMessage {
            role: match m.role {
                MessageRole::System => 1,
                MessageRole::User => 2,
                MessageRole::Assistant => 3,
                MessageRole::Tool => 4,
            },
            content: m
                .content
                .into_iter()
                .map(|p| match p {
                    ContentPart::Text { text } => ProtoContentPart {
                        part: Some(crate::proto::content_part::Part::Text(ProtoTextPart { text })),
                    },
                    ContentPart::Image { url } => ProtoContentPart {
                        part: Some(crate::proto::content_part::Part::Image(ProtoImagePart { url })),
                    },
                    ContentPart::Audio { url } => ProtoContentPart {
                        part: Some(crate::proto::content_part::Part::Audio(ProtoAudioPart { url })),
                    },
                    ContentPart::File { url, mime_type } => ProtoContentPart {
                        part: Some(crate::proto::content_part::Part::File(ProtoFilePart { url, mime_type })),
                    },
                })
                .collect(),
            name: m.name.unwrap_or_default(),
            tool_call_id: m.tool_call_id.unwrap_or_default(),
        })
        .collect();

    let tools = req
        .tools
        .into_iter()
        .map(|t| ProtoTool {
            name: t.name,
            description: t.description.unwrap_or_default(),
            parameters_json_schema: t
                .parameters_json_schema
                .map(|v| v.to_string())
                .unwrap_or_default(),
            strict: t.strict,
        })
        .collect();

    let constraints = ProtoGenerationConstraints {
        max_tokens: req.constraints.max_tokens.unwrap_or_default(),
        temperature: req.constraints.temperature.unwrap_or_default(),
        top_p: req.constraints.top_p.unwrap_or_default(),
        top_k: req.constraints.top_k.unwrap_or_default(),
        stop_sequences: req.constraints.stop_sequences,
        presence_penalty: req.constraints.presence_penalty.unwrap_or_default(),
        frequency_penalty: req.constraints.frequency_penalty.unwrap_or_default(),
        reasoning_effort: req.constraints.reasoning_effort.unwrap_or_default(),
        stream: req.constraints.stream,
    };

    CanonicalAiRequest {
        request_id: req.request_id.to_string(),
        agent_id: req.agent_id.unwrap_or_default(),
        session_id: req.session_id.unwrap_or_default(),
        messages,
        tools,
        tool_choice: req.tool_choice.unwrap_or_default(),
        constraints: Some(constraints),
        preferred_model: req.preferred_model.unwrap_or_default(),
        metadata: req.metadata.into_iter().collect(),
        response_format_json_schema: req.response_format.map(|v| v.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_starts_empty() {
        let st = AdapterRegistryState::new(RequestReplayConfig::default());
        let adapters = st.inner.adapters.read().await;
        assert!(adapters.is_empty());
    }
}
