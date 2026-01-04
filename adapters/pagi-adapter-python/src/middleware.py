"""Pure-ish middleware functions over canonical requests.

These are intended to be deterministic transforms (or annotated enrichments)
over the canonical request/response surface.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


def token_count(text: str) -> int:
    try:
        import tiktoken  # type: ignore

        enc = tiktoken.get_encoding("cl100k_base")
        return len(enc.encode(text))
    except Exception:
        # Fallback heuristic: split on whitespace.
        return len(text.split())


def iter_text_from_messages(messages) -> list[str]:
    """Extract all text fragments from protobuf `messages`.

    Supports multimodal messages by ignoring non-text parts.
    """
    out: list[str] = []
    for m in messages:
        for part in getattr(m, "content", []):
            # `ContentPart` uses a oneof named `part`; python stubs expose helpers.
            kind = part.WhichOneof("part")
            if kind == "text":
                out.append(part.text.text)
    return out


def token_count_messages(messages) -> int:
    return sum(token_count(t) for t in iter_text_from_messages(messages))


def model_route(preferred_model: str | None, tools_count: int) -> str:
    # If the client already hinted a model, honor it.
    if preferred_model:
        return preferred_model

    # Heuristic routing example.
    if tools_count > 0:
        return "gpt-4o-mini"  # tool-capable baseline
    return "gpt-4o-mini"


@dataclass(frozen=True)
class ModelInfo:
    provider: str
    model: str
    cost_input_per_1m: float
    cost_output_per_1m: float
    max_context: int
    capabilities: set[str]


# Minimal, hardcoded model catalog (can be made config-driven later).
MODELS: dict[str, ModelInfo] = {
    "gpt-4o-mini": ModelInfo(
        provider="openai",
        model="gpt-4o-mini",
        cost_input_per_1m=0.15,
        cost_output_per_1m=0.60,
        max_context=128_000,
        capabilities={"tools", "json_mode", "vision"},
    ),
    "claude-sonnet": ModelInfo(
        provider="anthropic",
        model="claude-sonnet",
        cost_input_per_1m=3.00,
        cost_output_per_1m=15.00,
        max_context=200_000,
        capabilities={"tools", "reasoning"},
    ),
}


def _needs_vision(messages) -> bool:
    for m in messages:
        for part in getattr(m, "content", []):
            kind = part.WhichOneof("part")
            if kind == "image" or kind == "audio" or kind == "file":
                return True
    return False


def route_to_model(req) -> str:
    """Route a protobuf CanonicalAIRequest to a model.

    This is a pure function of request fields (messages/tools/preferred_model).
    """

    preferred = getattr(req, "preferred_model", "") or None
    if preferred:
        return preferred

    tokens = token_count_messages(req.messages)
    needs_tools = bool(req.tools)
    needs_vision = _needs_vision(req.messages)

    candidates: list[ModelInfo] = []
    for m in MODELS.values():
        if tokens >= m.max_context:
            continue
        if needs_tools and "tools" not in m.capabilities:
            continue
        if needs_vision and "vision" not in m.capabilities:
            continue
        candidates.append(m)

    if not candidates:
        # Worst-case fallback.
        return "gpt-4o-mini"

    # Naive cost heuristic: treat output tokens as ~25% of input.
    est_output = max(64, int(tokens * 0.25))
    def score(m: ModelInfo) -> float:
        return m.cost_input_per_1m * (tokens / 1_000_000.0) + m.cost_output_per_1m * (est_output / 1_000_000.0)

    return min(candidates, key=score).model


def embed_cache(_key: str) -> bool:
    # Placeholder for Redis-backed embedding cache.
    return False
