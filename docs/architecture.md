# Architecture

PAGI is designed around a **canonical internal request format** handled by the Rust core.

## Why canonicalization?

Without a canonical request, every protocol handler would need bespoke translations for every adapter (N² complexity). With canonicalization:

- each protocol translates **once** into a canonical request
- each adapter implements **one** handler that consumes canonical requests

## Components

- [`pagi-gateway-core`](pagi-gateway-core/src/main.rs):
  - Ingress: REST/gRPC/GraphQL/WS
  - Auth, rate limiting, observability
  - Adapter registry and routing
- Adapters:
  - Python: AI middleware as pure functions
  - Go: agent workflows (DAGs)
  - Java: enterprise integrations

## CQRS and GraphQL

GraphQL is intended to be **read-heavy**. Commands flow through protocol ingress → canonical → adapter, while reads can be federated and cached.

