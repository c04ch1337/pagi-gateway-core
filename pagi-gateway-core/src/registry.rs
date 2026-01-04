use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::info;

use crate::canonical::{CanonicalAIRequest, Intent, Payload};
use crate::config::RequestReplayConfig;
use crate::proto::{
    adapter_registry_server::{AdapterRegistry, AdapterRegistryServer},
    adapter_service_client::AdapterServiceClient,
    AdapterCapabilities, AdapterInfo, CanonicalAiRequest, CanonicalAiResponse,
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
        let (adapter_id, adapter) = adapters
            .iter()
            .next()
            .map(|(k, v)| (k.clone(), v.clone()))
            .context("no adapters registered")?;

        drop(adapters);

        let mut client = AdapterServiceClient::connect(adapter.endpoint.clone()).await?;
        let proto_req: CanonicalAiRequest = to_proto(req);
        let resp: CanonicalAiResponse = client.process(proto_req).await?.into_inner();
        Ok(ForwardResponse { request_id: resp.request_id, adapter_id, json: resp.json })
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
    let intent: i32 = match req.intent {
        Intent::IntentUnspecified => 0,
        Intent::IntentChat => 1,
        Intent::IntentEmbed => 2,
        Intent::IntentTool => 3,
    };
    let payload = match req.payload {
        Payload::Text { text } => Some(crate::proto::canonical_ai_request::Payload::Text(crate::proto::TextPayload { text })),
        Payload::Json { json } => Some(crate::proto::canonical_ai_request::Payload::Json(crate::proto::JsonPayload { json: json.to_string() })),
    };
    CanonicalAiRequest {
        request_id: req.request_id.to_string(),
        agent_id: req.agent_id,
        intent,
        constraints: req.constraints.into_iter().collect(),
        payload,
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
