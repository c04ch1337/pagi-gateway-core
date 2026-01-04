from pydantic import BaseModel


class ProviderConfig(BaseModel):
    adapter_id: str = "ollama"
    bind: str = "127.0.0.1:6003"
    core_grpc: str = "127.0.0.1:50051"
    version: str = "0.1.0"
    base_url: str = "http://127.0.0.1:11434/v1"
    default_model: str = "llama3.2:3b"


def load_config() -> ProviderConfig:
    import os

    return ProviderConfig(
        adapter_id=os.getenv("PAGI_ADAPTER_ID", "ollama"),
        bind=os.getenv("PAGI_ADAPTER_BIND", "127.0.0.1:6003"),
        core_grpc=os.getenv("PAGI_CORE_GRPC", "127.0.0.1:50051"),
        version=os.getenv("PAGI_ADAPTER_VERSION", "0.1.0"),
        base_url=os.getenv("OLLAMA_BASE_URL", "http://127.0.0.1:11434/v1"),
        default_model=os.getenv("PAGI_DEFAULT_MODEL", "llama3.2:3b"),
    )

