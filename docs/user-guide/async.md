# Async

`aget_tile()` and `atile()` return native coroutines that you can await in any asyncio event
loop:

```python
tile = await server.atile("buildings", 14, 8710, 5935)
tiles = await asyncio.gather(
    *(server.aget_tile("world", 2, x, y) for x in range(4) for y in range(4))
)
```

The fetch is spawned onto the `TileServer`'s own Tokio runtime as soon as the coroutine is
first awaited. The coroutine then only waits for the result, so the event loop is never
blocked and needs no thread pool. Cancelling the coroutine drops the result, but a fetch
that has already started runs to completion on the runtime.

Loading a config is synchronous. Do it at startup, e.g. in a FastAPI lifespan handler, or
wrap it in `await asyncio.to_thread(martin_py.TileServer, path)`.
