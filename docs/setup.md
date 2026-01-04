# Setup (bare metal)

## Rust

```bash
curl https://sh.rustup.rs -sSf | sh
```

## Python

```bash
sudo apt-get update && sudo apt-get install -y python3-venv python3-pip

python3 -m venv .venv
. .venv/bin/activate
pip install -r adapters/pagi-adapter-python/requirements.txt
```

## Protobuf

Install `protoc` from your OS package manager.

## Run all

Use [`deployment/run.sh`](deployment/run.sh) to build and launch the core + adapters.
