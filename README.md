# martin-py

Serve [Martin](https://github.com/maplibre/martin) tile sources from Python, in-process.

`martin_py.TileServer` loads an ordinary Martin config, resolves every source once (exactly
like `martin` does at startup), and then answers what Martin's tile endpoints would, without
opening a network port or going through HTTP:

| Martin endpoint               | `martin_py.TileServer`                                   |
| ----------------------------- | -------------------------------------------------------- |
| `/catalog` (tiles)            | `catalog()` / `catalog_json()`                           |
| `/{source_ids}`               | `tilejson()` / `get_source_info()`                       |
| `/{source_ids}/{z}/{x}/{y}`   | `get_tile()` / `tile()`, async `aget_tile()` / `atile()` |

It binds the `TileServer` path of the [geobeyond/martin `duckdb_enhancement`
branch](https://github.com/geobeyond/martin/tree/duckdb_enhancement). That branch adds
DuckDB/GeoParquet sources, including remote `s3://` GeoParquet authenticated through the
object-store settings.

```python
import martin_py

server = martin_py.TileServer("martin-config.yaml")
# or: martin_py.TileServer.from_dict({"pmtiles": {"sources": {"world": "world.pmtiles"}}})
print(server.list_sources())

tilejson = server.tilejson("world", "https://example.com/tiles/world")
data, content_type, content_encoding = server.get_tile("world", 0, 0, 0)

# inside an asyncio event loop; the fetch runs on martin's own runtime
tile = await server.atile("world", 0, 0, 0)
```

`get_tile` returns the tile's bytes exactly as Martin would send them, so they are still
compressed when the source stores compressed tiles. It also returns the `Content-Type` and,
if there is one, the `Content-Encoding`. Empty bytes mean the source has no data for that
tile; Martin's HTTP server answers those requests with `204 No Content`.

Errors are subclasses of `martin_py.MartinError`: `ConfigError`, `UnknownSourceError`,
`TileRequestError` (a 4xx in Martin's terms) and `TileFetchError` (a 5xx). An impossible
`z/x/y` raises `ValueError`.

## Layout

The layout follows [mapli](../mapli):

```
martin-py/
├── Cargo.toml          # workspace root + the PyO3 crate (`martin_py._core`), deliberately thin
├── src/lib.rs          #   argument conversion, GIL release, error mapping
├── martin-py-core/     # pure-Rust crate with all the logic, tested with `cargo test`
├── src/martin_py/      # the Python package: `__init__.py`, `_core.pyi` stubs, `py.typed`
├── tests/              # pytest suite + data fixtures copied from martin's tests/fixtures
├── examples/           # FastAPI tile API
└── docs/               # zensical site
```

## Building

martin is a **path dependency** on the sibling checkout `../martin`. That checkout must be on
the `duckdb_enhancement` branch. To build without the checkout, switch to the git dependency
that is commented out in [Cargo.toml](Cargo.toml).

```bash
uv sync                  # builds the extension (debug) into .venv, with the dev group
uv run pytest            # Python tests
cargo test               # Rust tests (martin-py-core)
```

The first build compiles DuckDB from source (the `duckdb` feature, martin's `unstable-duckdb`).
It takes several minutes; later builds reuse Cargo's cache.

### A wheel for another project (e.g. fastgeoapi)

```bash
uv run maturin build --release --compatibility linux
# -> target/wheels/martin_py-0.1.0-cp311-abi3-linux_x86_64.whl
```

The wheel uses the stable ABI (`abi3`, CPython ≥ 3.11), so a single file works for every
interpreter version. `--compatibility linux` keeps the platform tag fixed. Without it, the tag
follows the build host's glibc (e.g. `manylinux_2_39`), which changes the filename and breaks
consumers that pin the wheel by path. After a rebuild, reinstall the wheel in the consuming
project, because its contents and hash have changed:

```bash
cd ../fastgeoapi
uv lock --upgrade-package martin-py
uv sync --reinstall-package martin-py
```

### Source kinds

Cargo features select which source kinds are compiled in. The default is `pmtiles`,
`processing` (GeoJSON) and `duckdb`, which need no system libraries. `mbtiles`, `postgres`
and `cog` are available too:

```bash
uv run maturin build --release --compatibility linux --features postgres,mbtiles
```

At least one source-kind feature must be enabled.

## Debugging

- `RUST_LOG=martin=debug,martin_core=debug` shows martin's logs on stderr. It is read when
  `martin_py` is first imported.
- `MARTIN_PY_POLL_DIRECTLY=1` polls the config load on the calling thread instead of a Tokio
  worker, so a debugger stepping through source resolution stays on one thread.
