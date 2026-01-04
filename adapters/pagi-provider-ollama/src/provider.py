import json
import time


def _import_contracts():
    from pagi_contracts import agent_pb2  # type: ignore

    return agent_pb2


def _message_role_to_openai(role: int) -> str:
    return {
        1: "system",
        2: "user",
        3: "assistant",
        4: "tool",
    }.get(int(role), "user")


def normalize_messages(messages):
    out = []
    for m in messages:
        parts = []
        for p in m.content:
            kind = p.WhichOneof("part")
            if kind == "text":
                parts.append({"type": "text", "text": p.text.text})
            elif kind == "image":
                parts.append({"type": "image_url", "image_url": {"url": p.image.url}})
            elif kind == "audio":
                parts.append({"type": "text", "text": f"[audio] {p.audio.url}"})
            elif kind == "file":
                parts.append({"type": "text", "text": f"[file {p.file.mime_type}] {p.file.url}"})

        if not parts:
            parts = [{"type": "text", "text": ""}]

        out.append({"role": _message_role_to_openai(m.role), "content": parts})
    return out


def normalize_tools(tools):
    out = []
    for t in tools:
        schema = {}
        if getattr(t, "parameters_json_schema", ""):
            try:
                schema = json.loads(t.parameters_json_schema)
            except Exception:
                schema = {}
        out.append(
            {
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description or "",
                    "parameters": schema,
                },
            }
        )
    return out


def map_model(req, default_model: str) -> str:
    # Resolve model: metadata.ollama_model > metadata.routed_model > preferred_model > default.
    try:
        if req.metadata.get("ollama_model"):
            return req.metadata.get("ollama_model")
        if req.metadata.get("routed_model"):
            # If an OpenRouter-style slug was provided, you can map it externally.
            return req.metadata.get("routed_model")
    except Exception:
        pass

    return (req.preferred_model or None) or default_model


async def call_ollama(req, *, base_url: str, default_model: str):
    agent_pb2 = _import_contracts()

    from openai import AsyncOpenAI

    client = AsyncOpenAI(api_key="ollama", base_url=base_url)

    model = map_model(req, default_model)
    messages = normalize_messages(req.messages)
    tools = normalize_tools(req.tools)

    kwargs = {"model": model, "messages": messages}
    if tools:
        kwargs["tools"] = tools
        kwargs["tool_choice"] = req.tool_choice or "auto"

    if req.constraints.max_tokens:
        kwargs["max_tokens"] = int(req.constraints.max_tokens)
    if req.constraints.temperature:
        kwargs["temperature"] = float(req.constraints.temperature)

    started = time.time()
    resp = await client.chat.completions.create(**kwargs)
    latency_ms = int((time.time() - started) * 1000)

    text = ""
    try:
        text = resp.choices[0].message.content or ""
    except Exception:
        text = ""

    payload = {
        "provider": "ollama",
        "model": model,
        "actual_model": getattr(resp, "model", None),
        "latency_ms": latency_ms,
        "text": text,
    }

    return agent_pb2.CanonicalAIResponse(request_id=req.request_id, adapter_id="ollama", json=json.dumps(payload))

