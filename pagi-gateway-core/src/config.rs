use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub version: String,
    pub core: CoreConfig,
    #[serde(default)]
    pub adapters: Vec<AdapterConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreConfig {
    pub bind_http: String,
    pub bind_grpc: String,
    #[serde(default)]
    pub request_replay: RequestReplayConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ObservabilityConfig {
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdapterConfig {
    pub id: String,
    pub kind: String,
    pub endpoint: String,
    #[serde(default)]
    pub capabilities: AdapterCapabilities,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AdapterCapabilities {
    #[serde(default)]
    pub streaming: bool,
    #[serde(default)]
    pub token_count: bool,
    #[serde(default)]
    pub model_route: bool,
    #[serde(default)]
    pub embed_cache: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestReplayConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_replay_path")]
    pub path: String,
}

fn default_replay_path() -> String {
    "./replay.log".to_string()
}

impl Default for RequestReplayConfig {
    fn default() -> Self {
        Self { enabled: false, path: default_replay_path() }
    }
}

impl Config {
    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(serde_yaml::from_slice(&bytes)?)
    }
}

