# Development

## Layout

- `martin-py-core/`: a pure-Rust crate holding all the logic. It loads the config through
  martin's `read_config` / `parse_config`, `finalize` and `resolve`, and serves tiles through
  `martin::srv::DynTileSource` and `merge_tilejson`. Test it with `cargo test`.
- `src/lib.rs`: the PyO3 crate, built as `martin_py._core`. It only converts arguments,
  releases the GIL and maps `MartinPyError` variants to Python exceptions.
- `src/martin_py/`: the Python package. `TileServer` subclasses `_core.TileServer` and adds
  the dict-returning helpers and the alternative constructors. `_core.pyi` holds the stubs
  for the compiled module. The API reference is generated from it, so the docs build does not
  need the extension to be compiled.

## The martin dependency

martin is a crates.io dependency, pinned to an exact version. `martin::srv` is martin's HTTP layer, not a stable library API,
so after bumping the version, run `cargo check` here first. All martin types are confined to
`martin-py-core`, so only that crate needs to change when martin's API moves.

`Cargo.lock` was seeded from martin's own lock file, so this workspace builds the same
dependency versions as martin.

## Common commands

```bash
uv sync                       # build the extension into .venv (debug profile)
uv run pytest                 # Python tests
cargo test                    # Rust tests
prek run --all-files          # fmt, clippy, ruff, ty (see prek.toml)
uv run maturin build --release --compatibility linux   # wheel for local consumers
uv run --group docs zensical serve                     # docs
```
