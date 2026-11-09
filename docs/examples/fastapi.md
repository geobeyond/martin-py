# FastAPI tile API

A [FastAPI] application exposing Martin's `/catalog`, `/{source_ids}` and
`/{source_ids}/{z}/{x}/{y}` routes, backed by `martin_py`.

- The `TileServer` is created in the lifespan handler, so the sources are resolved once and
  shared by all requests.
- The tile route uses `aget_tile`, so the event loop keeps serving other requests while a tile
  is fetched.

```shell
MARTIN_CONFIG=martin-config.yaml uv run fastapi dev examples/webapi.py
```

````python title="examples/webapi.py"
--8<-- "examples/webapi.py"
````

[FastAPI]: https://fastapi.tiangolo.com/
