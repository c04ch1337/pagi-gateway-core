use std::net::SocketAddr;

use anyhow::Context;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use tracing::{error, info};

use pagi_gateway_core::config::Config;
use pagi_gateway_core::middleware::observability::Metrics;
use pagi_gateway_core::protocols::{graphql, rest};
use pagi_gateway_core::registry::{AdapterRegistryState, AdapterRegistrySvc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config_path = parse_config_path_from_args()
        .or_else(|| std::env::var("PAGI_CONFIG").ok())
        .unwrap_or_else(|| "../config/pagi.yaml".to_string());
    let cfg = Config::from_path(&config_path).context("loading config")?;
    info!(%config_path, "loaded config");

    let metrics = Metrics::new();
    let registry_state = AdapterRegistryState::new(cfg.core.request_replay.clone());
    let registry_state_for_http = registry_state.clone();

    let http_addr: SocketAddr = cfg.core.bind_http.parse().context("invalid core.bind_http")?;
    let grpc_addr: SocketAddr = cfg.core.bind_grpc.parse().context("invalid core.bind_grpc")?;

    let graphql_schema = graphql::build_schema(registry_state.clone());

    let make_svc = make_service_fn(move |_conn| {
        let registry_state = registry_state_for_http.clone();
        let metrics = metrics.clone();
        let graphql_schema = graphql_schema.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let registry_state = registry_state.clone();
                let metrics = metrics.clone();
                let graphql_schema = graphql_schema.clone();
                async move { handle_http(req, registry_state, metrics, graphql_schema).await }
            }))
        }
    });

    let http_server = Server::bind(&http_addr).serve(make_svc);
    info!(%http_addr, "http listening");

    let grpc_server = tonic::transport::Server::builder()
        .add_service(AdapterRegistrySvc::new(registry_state.clone()))
        .serve(grpc_addr);
    info!(%grpc_addr, "grpc listening");

    tokio::select! {
        r = http_server => {
            if let Err(e) = r { error!(error=%e, "http server error"); }
        }
        r = grpc_server => {
            if let Err(e) = r { error!(error=%e, "grpc server error"); }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("shutdown requested");
        }
    }

    Ok(())
}

async fn handle_http(
    req: Request<Body>,
    registry: AdapterRegistryState,
    metrics: Metrics,
    graphql_schema: graphql::SchemaType,
) -> Result<Response<Body>, hyper::Error> {
    match (req.method().as_str(), req.uri().path()) {
        ("GET", "/healthz") => Ok(Response::new(Body::from("ok"))),
        ("GET", "/metrics") => Ok(metrics.render()),
        ("POST", "/v1/ai:call") | ("POST", "/api/call") => rest::handle_call(req, registry, metrics).await,
        ("GET", "/graphql") | ("POST", "/graphql") => graphql::handle(req, graphql_schema).await,
        _ => {
            let mut r = Response::new(Body::from("not found"));
            *r.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(r)
        }
    }
}

fn parse_config_path_from_args() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--config" {
            return args.next();
        }
        if let Some(v) = a.strip_prefix("--config=") {
            return Some(v.to_string());
        }
    }
    None
}
