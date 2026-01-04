# PAGI

Polyglot AI Gateway Infrastructure (PAGI) is a research-friendly, production-capable AI gateway.

**Core idea:** [`pagi-gateway-core`](pagi-gateway-core/Cargo.toml) (Rust) is the **single ingress** and authoritative system boundary. Other languages run as **sidecar adapters** that register with the Rust core (gRPC or Unix sockets).

## Architecture (high level)

```mermaid
flowchart LR
  C[Clients
  REST / gRPC / GraphQL / WS] --> CORE[pagi-gateway-core (Rust)
  auth + rate limit + observability
  canonical request]
  CORE --> REG[Adapter Registry]
  REG --> PY[pagi-adapter-python
  AI middleware (pure functions)]
  REG --> GO[pagi-adapter-go
  workflow/agents]
  REG --> JAVA[pagi-adapter-java
  enterprise integration]
  CORE --> METRICS[/metrics (Prometheus)]
  CORE --> TRACES[Jaeger (OTLP)
  optional]
```

## Repository layout

- [`contracts/`](contracts/agent.proto) — shared Protobuf IDLs
- [`pagi-gateway-core/`](pagi-gateway-core/src/main.rs) — Rust ingress core
- [`adapters/`](adapters/) — language adapters/plugins
- [`config/`](config/pagi.yaml) — example unified YAML config
- [`tools/`](tools/) — shared scripts (protobuf generation)
- [`deployment/`](deployment/run.sh) — bare-metal run scripts (Docker is optional and isolated)
- [`docs/`](docs/architecture.md) — deeper documentation

## Quickstart (bare metal)

### Prereqs

- Rust (via rustup)
- Python 3.11+ (recommended) with venv + pip (`python3-venv` / `python3-pip` on Debian/Ubuntu)
- Protobuf compiler (`protoc`) if you want to regenerate stubs

### Run

1) Generate protobuf stubs (optional; Rust uses `tonic-build` at compile time):

```bash
./tools/generate-protos.sh
```

2) Start the Rust core:

```bash
cd pagi-gateway-core
cargo run
```

3) In another terminal, start the Python adapter:

```bash
cd adapters/pagi-adapter-python
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
python -m src.main
```

4) Call the REST ingress endpoint:

```bash
curl -sS -X POST http://127.0.0.1:8282/v1/ai:call \
  -H 'content-type: application/json' \
  -d '{"agent_id":"demo","intent":"INTENT_CHAT","payload":{"text":"hello"}}'
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).
