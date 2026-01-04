from pydantic import BaseModel


class ProviderConfig(BaseModel):
    adapter_id: str = "openrouter"
    bind: str = "127.0.0.1:6002"
    core_grpc: str = "127.0.0.1:50051"
    version: str = "0.1.0"
    default_model: str = "anthropic/claude-3.5-sonnet"
    base_url: str = "https://openrouter.ai/api/v1"


def load_config() -> ProviderConfig:
    import os

    return ProviderConfig(
        adapter_id=os.getenv("PAGI_ADAPTER_ID", "openrouter"),
        bind=os.getenv("PAGI_ADAPTER_BIND", "127.0.0.1:6002"),
        core_grpc=os.getenv("PAGI_CORE_GRPC", "127.0.0.1:50051"),
        version=os.getenv("PAGI_ADAPTER_VERSION", "0.1.0"),
        default_model=os.getenv("PAGI_DEFAULT_MODEL", "anthropic/claude-3.5-sonnet"),
        base_url=os.getenv("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1"),
    )

