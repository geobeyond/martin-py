# Serving tiles

```python
server.list_sources()  # ["buildings", "world"]
server.catalog()  # {"buildings": {...}, "world": {...}}
server.tilejson("buildings", "https://x/t/buildings")
data, content_type, content_encoding = server.get_tile("buildings", 14, 8710, 5935)
```

`source_ids` accepts a comma-joined list such as `"buildings,roads"`, or an alias, the same
way Martin's URLs do. Martin concatenates the vector tiles of all the listed sources into one
tile. The sources must share a tile format and grid; if they don't, the call raises
`TileRequestError`.

## What a tile looks like

- `data` is the bytes Martin would put in the HTTP body. PMTiles and MBTiles sources often
  store gzip-compressed vector tiles, and those bytes are returned still compressed, with
  `content_encoding == "gzip"`. Tiles generated on the fly (GeoJSON, DuckDB, PostGIS) are not
  compressed.
- Empty `data` means the source has no features there. Martin's HTTP server answers those
  requests with `204 No Content`.
- `server.tile(...)` returns the same values as a `Tile` named tuple
  (`tile.data`, `tile.content_type`, `tile.content_encoding`).

## Errors

| Exception            | When                                                | HTTP equivalent |
| -------------------- | --------------------------------------------------- | --------------- |
| `ValueError`         | `z/x/y` is not a tile on the zoom level             | 400             |
| `UnknownSourceError` | a source id is not registered                       | 404             |
| `TileRequestError`   | zoom out of range, sources that cannot be merged, bad query | 400 / 404 / 406 |
| `TileFetchError`     | the source itself failed                            | 500             |

## Threads

`TileServer` can be shared between threads. Every call that does I/O releases the GIL, and the
sources run on the server's own Tokio runtime. Its size is set with
`TileServer(..., worker_threads=n)` and defaults to one thread per core.
