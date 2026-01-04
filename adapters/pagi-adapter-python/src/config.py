from pydantic import BaseModel


class AdapterConfig(BaseModel):
    adapter_id: str = "python"
    bind: str = "127.0.0.1:6000"
    core_grpc: str = "127.0.0.1:50051"
    version: str = "0.1.0"


def load_config() -> AdapterConfig:
    import os

    return AdapterConfig(
        adapter_id=os.getenv("PAGI_ADAPTER_ID", "python"),
        bind=os.getenv("PAGI_ADAPTER_BIND", "127.0.0.1:6000"),
        core_grpc=os.getenv("PAGI_CORE_GRPC", "127.0.0.1:50051"),
        version=os.getenv("PAGI_ADAPTER_VERSION", "0.1.0"),
    )

