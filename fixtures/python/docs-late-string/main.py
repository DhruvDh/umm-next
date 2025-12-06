def foo():
    x = 1
    "not a docstring"
    return x


class Bar:
    value = 1
    "also not a docstring"

    def __init__(self):
        pass
