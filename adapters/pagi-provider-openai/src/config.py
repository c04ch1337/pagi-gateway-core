from pydantic import BaseModel


class ProviderConfig(BaseModel):
    adapter_id: str = "openai"
    bind: str = "127.0.0.1:6001"
    core_grpc: str = "127.0.0.1:50051"
    version: str = "0.1.0"
    default_model: str = "gpt-4o-mini"


def load_config() -> ProviderConfig:
    import os

    return ProviderConfig(
        adapter_id=os.getenv("PAGI_ADAPTER_ID", "openai"),
        bind=os.getenv("PAGI_ADAPTER_BIND", "127.0.0.1:6001"),
        core_grpc=os.getenv("PAGI_CORE_GRPC", "127.0.0.1:50051"),
        version=os.getenv("PAGI_ADAPTER_VERSION", "0.1.0"),
        default_model=os.getenv("PAGI_DEFAULT_MODEL", "gpt-4o-mini"),
    )

