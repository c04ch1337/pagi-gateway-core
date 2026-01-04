from src.middleware import model_route, route_to_model, token_count


class _Part:
    def __init__(self, kind: str, text: str | None = None, url: str | None = None):
        self._kind = kind
        if kind == "text":
            self.text = type("T", (), {"text": text or ""})()
        if kind == "image":
            self.image = type("I", (), {"url": url or ""})()

    def WhichOneof(self, _name: str) -> str:
        return self._kind


class _Msg:
    def __init__(self, parts):
        self.content = parts


class _Req:
    def __init__(self, messages, tools=None, preferred_model: str = ""):
        self.messages = messages
        self.tools = tools or []
        self.preferred_model = preferred_model


def test_model_route_chat():
    assert model_route(None, 0)


def test_route_to_model_honors_preferred_model():
    req = _Req(messages=[_Msg([_Part("text", text="hi")])], preferred_model="claude-sonnet")
    assert route_to_model(req) == "claude-sonnet"


def test_token_count_nonzero():
    assert token_count("hello world") >= 1
