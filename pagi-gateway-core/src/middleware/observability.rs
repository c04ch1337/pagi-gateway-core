use std::sync::Arc;

use hyper::{Body, Response};
use prometheus::{Encoder, HistogramOpts, HistogramVec, IntCounterVec, Registry, TextEncoder};

#[derive(Clone)]
pub struct Metrics {
    inner: Arc<Inner>,
}

struct Inner {
    registry: Registry,
    pub requests_total: IntCounterVec,
    pub request_latency: HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = IntCounterVec::new(
            prometheus::Opts::new("pagi_requests_total", "Total requests"),
            &["protocol", "status"],
        )
        .expect("metric");
        let request_latency = HistogramVec::new(
            HistogramOpts::new("pagi_request_latency_seconds", "Request latency"),
            &["protocol"],
        )
        .expect("metric");

        registry.register(Box::new(requests_total.clone())).expect("register");
        registry
            .register(Box::new(request_latency.clone()))
            .expect("register");

        Self {
            inner: Arc::new(Inner { registry, requests_total, request_latency }),
        }
    }

    pub fn render(&self) -> Response<Body> {
        let mf = self.inner.registry.gather();
        let mut buf = Vec::new();
        TextEncoder::new().encode(&mf, &mut buf).expect("encode");

        Response::builder()
            .header("content-type", TextEncoder::new().format_type())
            .body(Body::from(buf))
            .unwrap()
    }

    pub fn inc_requests(&self, protocol: &'static str, status: &'static str) {
        self.inner.requests_total.with_label_values(&[protocol, status]).inc();
    }

    pub fn observe_latency(&self, protocol: &'static str, seconds: f64) {
        self.inner.request_latency.with_label_values(&[protocol]).observe(seconds);
    }
}
