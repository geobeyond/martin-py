//! In-process access to Martin's **tile endpoints**, without going through HTTP.
//!
//! This is the Rust half of martin-py: the PyO3 crate at the workspace root only converts
//! arguments, releases the GIL and maps errors. All logic is here, testable with `cargo test`.
//!
//! It mirrors what [`martin::srv`] serves over HTTP by calling the same source-resolution and
//! tile-fetching code directly:
//! - `/catalog` (tiles) -> [`TileServer::catalog_json`]
//! - `/{source_ids}` -> [`TileServer::tilejson_json`]
//! - `/{source_ids}/{z}/{x}/{y}` -> [`TileServer::tile`] / [`TileServer::tile_async`]
//!
//! Sources are loaded once from a Martin config when the [`TileServer`] is opened, exactly
//! like the `martin` binary does at startup. At least one source-kind feature (`pmtiles`,
//! `processing`, `duckdb`, `mbtiles`, `postgres`, `cog`) must be enabled.
mod error;
mod server;
mod types;

pub use error::{MartinPyError, Result};
pub use server::{TileServer, load_tile_manager};
pub use types::{ConfigSource, POLL_DIRECTLY_ENV, ServerOptions, Tile, TileCoord, TileData};
