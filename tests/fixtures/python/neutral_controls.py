async def neutral(value, items, resource):
    async with resource:
        match value:
            case _ if value and (items or not resource):
                pass
    async for item in items:
        pass
    else:
        pass
