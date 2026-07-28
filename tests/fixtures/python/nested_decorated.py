def outer(flag):
    @decorator(flag and flag)
    def inner():
        pass

    @decorator(flag if flag else flag)
    def inner_two():
        pass

    return inner, inner_two
