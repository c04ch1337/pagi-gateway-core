import json
import os
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


async def call_openrouter(req, *, default_model: str, base_url: str):
    agent_pb2 = _import_contracts()

    # Resolve model: metadata.routed_model > preferred_model > default.
    routed_model = None
    try:
        routed_model = req.metadata.get("routed_model")
    except Exception:
        routed_model = None

    model = routed_model or (req.preferred_model or None) or default_model

    api_key = os.getenv("OPENROUTER_API_KEY")
    if not api_key:
        payload = {
            "provider": "openrouter",
            "model": model,
            "note": "OPENROUTER_API_KEY not set; returning stub response",
        }
        return agent_pb2.CanonicalAIResponse(request_id=req.request_id, adapter_id="openrouter", json=json.dumps(payload))

    from openai import AsyncOpenAI

    client = AsyncOpenAI(api_key=api_key, base_url=base_url)

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

    headers = {
        "HTTP-Referer": os.getenv("OPENROUTER_HTTP_REFERER", "http://localhost:8282"),
        "X-Title": os.getenv("OPENROUTER_APP_TITLE", "PAGI Gateway"),
    }

    started = time.time()
    resp = await client.chat.completions.create(extra_headers=headers, **kwargs)
    latency_ms = int((time.time() - started) * 1000)

    text = ""
    try:
        text = resp.choices[0].message.content or ""
    except Exception:
        text = ""

    payload = {
        "provider": "openrouter",
        "model": model,
        "actual_model": getattr(resp, "model", None),
        "latency_ms": latency_ms,
        "text": text,
    }

    return agent_pb2.CanonicalAIResponse(request_id=req.request_id, adapter_id="openrouter", json=json.dumps(payload))

