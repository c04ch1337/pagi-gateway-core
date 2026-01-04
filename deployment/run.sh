#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[pagi] building rust core"
(cd "$ROOT_DIR/pagi-gateway-core" && cargo build)

echo "[pagi] starting rust core (in background)"
(cd "$ROOT_DIR/pagi-gateway-core" && cargo run) &
CORE_PID=$!

sleep 1

echo "[pagi] starting python adapter"
cd "$ROOT_DIR/adapters/pagi-adapter-python"
python3 -m venv .venv >/dev/null 2>&1 || true
source .venv/bin/activate
pip install -r requirements.txt >/dev/null
python -m src.main &
PY_PID=$!

trap "kill $PY_PID $CORE_PID 2>/dev/null || true" EXIT

echo "[pagi] running. core pid=$CORE_PID python pid=$PY_PID"
wait
