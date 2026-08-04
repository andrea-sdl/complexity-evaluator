@decorator
async def decorated(é):
    if é:
        pass


class Café:
    @decorator
    def method(self):
        return lambda value: value if value else 0


def outer(flag):
    if flag:
        def nested(value):
            if value:
                pass
        callback = lambda value: value if value else 0
