from src.middleware import model_route, token_count


def test_model_route_chat():
    assert model_route(None, 0)


def test_token_count_nonzero():
    assert token_count("hello world") >= 1
