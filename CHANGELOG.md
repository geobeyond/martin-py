# Changelog

## 0.1.0 (unreleased)

First version as a standalone project, rebuilt from the `martin-py` crate of the
`pyO3_bindings` martin branch. It now binds the `duckdb_enhancement` branch.

- Split into `martin-py-core` (Rust logic) and a thin PyO3 crate, following mapli's layout.
- `martin_py` is now a Python package (`martin_py._core` extension, typed with `py.typed`).
- `TileServer` can be constructed from a file, `from_yaml()` or `from_dict()`, and takes
  `worker_threads`.
- New: `catalog()`, `tilejson()`, the `Tile` named tuple, and async `aget_tile()` / `atile()`.
- Dedicated exceptions under `MartinError`: `ConfigError`, `UnknownSourceError`,
  `TileRequestError`, `TileFetchError`. They used to be `RuntimeError` / `ValueError`.
  An invalid `z/x/y` is still a `ValueError`.
- Stable-ABI wheel (`cp311-abi3`).
