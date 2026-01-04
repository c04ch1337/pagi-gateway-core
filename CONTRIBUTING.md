# Contributing

## Ground rules

- Keep the Rust core authoritative: adapters register with core; they are not peers.
- Prefer backwards-compatible changes to protobuf contracts.
- Keep containerization optional and isolated under [`deployment/`](deployment/).

## Development

- Format Rust: `cargo fmt`
- Lint Rust: `cargo clippy`
- Test Rust: `cargo test`
- Test Python adapter: `pytest`

## PR expectations

- Add/adjust tests where appropriate.
- Update docs when changing public-facing behavior.
- Keep config examples in [`config/pagi.yaml`](config/pagi.yaml) in sync with the code.

