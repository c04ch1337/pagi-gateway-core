#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACTS_DIR="$ROOT_DIR/contracts"
PYTHON_BIN="${PYTHON_BIN:-python3}"

echo "[pagi] contracts: $CONTRACTS_DIR"

PROTOC_AVAILABLE=1
if ! command -v protoc >/dev/null 2>&1; then
  PROTOC_AVAILABLE=0
  echo "[pagi] protoc not found: will generate python stubs only (go/java skipped)"
fi

PROTO_FILES=(
  "$CONTRACTS_DIR/agent.proto"
  "$CONTRACTS_DIR/model.proto"
  "$CONTRACTS_DIR/memory.proto"
)

### Python
PY_OUT="$ROOT_DIR/adapters/pagi-adapter-python/src/pagi_contracts"
mkdir -p "$PY_OUT"
if "$PYTHON_BIN" -c "import grpc_tools" >/dev/null 2>&1; then
  echo "[pagi] generating python stubs -> $PY_OUT"
  "$PYTHON_BIN" -m grpc_tools.protoc \
    -I"$CONTRACTS_DIR" \
    --python_out="$PY_OUT" \
    --grpc_python_out="$PY_OUT" \
    "${PROTO_FILES[@]}"
  touch "$PY_OUT/__init__.py"
else
  echo "[pagi] skip python stub generation (missing grpcio-tools)"
fi

### Go (optional)
GO_OUT="$ROOT_DIR/contracts/gen/go"
mkdir -p "$GO_OUT"
if [[ "$PROTOC_AVAILABLE" -eq 1 ]] && command -v protoc-gen-go >/dev/null 2>&1 && command -v protoc-gen-go-grpc >/dev/null 2>&1; then
  echo "[pagi] generating go stubs -> $GO_OUT"
  protoc -I"$CONTRACTS_DIR" \
    --go_out="$GO_OUT" --go_opt=paths=source_relative \
    --go-grpc_out="$GO_OUT" --go-grpc_opt=paths=source_relative \
    "${PROTO_FILES[@]}"
else
  echo "[pagi] skip go stub generation (missing protoc-gen-go / protoc-gen-go-grpc)"
fi

### Java (optional)
JAVA_OUT="$ROOT_DIR/contracts/gen/java"
mkdir -p "$JAVA_OUT"
if [[ "$PROTOC_AVAILABLE" -eq 1 ]] && command -v protoc-gen-grpc-java >/dev/null 2>&1; then
  echo "[pagi] generating java stubs -> $JAVA_OUT"
  protoc -I"$CONTRACTS_DIR" \
    --java_out="$JAVA_OUT" \
    --grpc-java_out="$JAVA_OUT" \
    "${PROTO_FILES[@]}"
else
  echo "[pagi] skip java stub generation (missing protoc-gen-grpc-java)"
fi

echo "[pagi] done"
