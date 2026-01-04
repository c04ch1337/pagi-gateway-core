"""Pure-ish middleware functions over canonical requests.

These are intended to be deterministic transforms (or annotated enrichments)
over the canonical request/response surface.
"""


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


def embed_cache(_key: str) -> bool:
    # Placeholder for Redis-backed embedding cache.
    return False
