import asyncio
import json
import logging

import grpc

from .config import load_config
from .middleware import iter_text_from_messages, model_route, token_count_messages


log = logging.getLogger("pagi.adapter.python")


def _import_contracts():
    """Import generated protobuf stubs.

    Expected location: adapters/pagi-adapter-python/src/pagi_contracts/
      - agent_pb2.py
      - agent_pb2_grpc.py
    """

    from pagi_contracts import agent_pb2, agent_pb2_grpc  # type: ignore

    return agent_pb2, agent_pb2_grpc


class AdapterService:
    def __init__(self, adapter_id: str):
        self.adapter_id = adapter_id

    async def Process(self, request, context):  # noqa: N802 (grpc style)
        agent_pb2, _ = _import_contracts()

        texts = iter_text_from_messages(request.messages)

        preferred_model = request.preferred_model or None
        chosen_model = model_route(preferred_model, len(request.tools))

        out = {
            "request_id": request.request_id,
            "agent_id": request.agent_id or None,
            "session_id": request.session_id or None,
            "model": chosen_model,
            "token_count": token_count_messages(request.messages),
            "constraints": {
                "max_tokens": request.constraints.max_tokens,
                "temperature": request.constraints.temperature,
                "top_p": request.constraints.top_p,
                "top_k": request.constraints.top_k,
                "stop_sequences": list(request.constraints.stop_sequences),
                "presence_penalty": request.constraints.presence_penalty,
                "frequency_penalty": request.constraints.frequency_penalty,
                "reasoning_effort": request.constraints.reasoning_effort,
                "stream": request.constraints.stream,
            },
            "tools": [
                {
                    "name": t.name,
                    "description": t.description or None,
                    "strict": bool(t.strict),
                }
                for t in request.tools
            ],
            "metadata": dict(request.metadata),
            "echo_text": "\n".join(texts) if texts else None,
        }

        return agent_pb2.CanonicalAIResponse(
            request_id=request.request_id,
            adapter_id=self.adapter_id,
            json=json.dumps(out),
        )


async def register_with_core(cfg) -> None:
    agent_pb2, agent_pb2_grpc = _import_contracts()

    channel = grpc.aio.insecure_channel(cfg.core_grpc)
    stub = agent_pb2_grpc.AdapterRegistryStub(channel)

    req = agent_pb2.RegisterAdapterRequest(
        adapter_id=cfg.adapter_id,
        endpoint=f"http://{cfg.bind}",
        capabilities=agent_pb2.AdapterCapabilities(
            streaming=False,
            token_count=True,
            model_route=True,
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
    log.info("starting python adapter id=%s bind=%s core_grpc=%s", cfg.adapter_id, cfg.bind, cfg.core_grpc)

    agent_pb2, agent_pb2_grpc = _import_contracts()

    server = grpc.aio.server()
    agent_pb2_grpc.add_AdapterServiceServicer_to_server(AdapterService(cfg.adapter_id), server)
    server.add_insecure_port(cfg.bind)

    await server.start()
    await register_with_core(cfg)
    log.info("registered with core")

    await server.wait_for_termination()


if __name__ == "__main__":
    asyncio.run(serve())
