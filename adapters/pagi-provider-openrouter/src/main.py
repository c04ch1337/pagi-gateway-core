import asyncio
import logging

import grpc

from .config import load_config
from .provider import call_openrouter


log = logging.getLogger("pagi.provider.openrouter")


def _import_contracts():
    from pagi_contracts import agent_pb2, agent_pb2_grpc  # type: ignore

    return agent_pb2, agent_pb2_grpc


class ProviderService:
    def __init__(self, cfg):
        self.cfg = cfg

    async def Process(self, request, context):  # noqa: N802
        return await call_openrouter(request, default_model=self.cfg.default_model, base_url=self.cfg.base_url)


async def register_with_core(cfg) -> None:
    agent_pb2, agent_pb2_grpc = _import_contracts()

    channel = grpc.aio.insecure_channel(cfg.core_grpc)
    stub = agent_pb2_grpc.AdapterRegistryStub(channel)

    req = agent_pb2.RegisterAdapterRequest(
        adapter_id=cfg.adapter_id,
        endpoint=f"http://{cfg.bind}",
        capabilities=agent_pb2.AdapterCapabilities(
            streaming=False,
            token_count=False,
            model_route=False,
            embed_cache=False,
        ),
        version=cfg.version,
    )
    resp = await stub.Register(req)
    if not resp.ok:
        raise RuntimeError("core rejected adapter registration")


async def serve() -> None:
    logging.basicConfig(level=logging.INFO)
    cfg = load_config()
    log.info("starting openrouter provider adapter id=%s bind=%s core_grpc=%s", cfg.adapter_id, cfg.bind, cfg.core_grpc)

    agent_pb2, agent_pb2_grpc = _import_contracts()

    server = grpc.aio.server()
    agent_pb2_grpc.add_AdapterServiceServicer_to_server(ProviderService(cfg), server)
    server.add_insecure_port(cfg.bind)

    await server.start()
    await register_with_core(cfg)
    log.info("registered with core")

    await server.wait_for_termination()


if __name__ == "__main__":
    asyncio.run(serve())

