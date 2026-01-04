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


def model_route(intent: int) -> str:
    # Keep consistent with Intent enum in contracts/agent.proto.
    # 1 = CHAT, 2 = EMBED, 3 = TOOL.
    if intent == 1:
        return "gpt-4o-mini"
    if intent == 2:
        return "text-embedding-3-small"
    if intent == 3:
        return "tool-executor"
    return "unknown"


def embed_cache(_key: str) -> bool:
    # Placeholder for Redis-backed embedding cache.
    return False

