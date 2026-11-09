//! Plain data types of the martin-py-core API.
use std::path::{Path, PathBuf};

pub use martin_tile_utils::{TileCoord, TileData};

/// Where a [`TileServer`](crate::TileServer) reads its Martin config from.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// A Martin config file on disk, exactly what `martin --config <path>` reads.
    Path(PathBuf),
    /// Martin config YAML (or JSON, which is valid YAML) held in memory.
    ///
    /// `file_name` is only used to label parse errors; relative paths inside the config are
    /// resolved against the process's working directory, as they would be for a file.
    Yaml {
        contents: String,
        file_name: PathBuf,
    },
}

impl ConfigSource {
    /// Config YAML held in memory, labelled `<memory>` in error messages.
    #[must_use]
    pub fn yaml(contents: impl Into<String>) -> Self {
        Self::Yaml {
            contents: contents.into(),
            file_name: PathBuf::from("<memory>"),
        }
    }
}

impl From<&Path> for ConfigSource {
    fn from(path: &Path) -> Self {
        Self::Path(path.to_owned())
    }
}

impl From<PathBuf> for ConfigSource {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

/// How a [`TileServer`](crate::TileServer) drives martin's async code.
#[derive(Debug, Clone, Default)]
pub struct ServerOptions {
    /// Worker threads of the server's Tokio runtime; `None` uses Tokio's default (one per core).
    pub worker_threads: Option<usize>,
    /// Poll the config load on the calling thread instead of through `Runtime::block_on`.
    ///
    /// A debugging aid: a debugger stepping through source resolution stays on the calling
    /// thread instead of disappearing into Tokio's scheduler. See [`ServerOptions::from_env`].
    pub poll_directly: bool,
}

/// Set to anything but empty or `0` to enable [`ServerOptions::poll_directly`].
pub const POLL_DIRECTLY_ENV: &str = "MARTIN_PY_POLL_DIRECTLY";

impl ServerOptions {
    /// Defaults, with `poll_directly` taken from the `MARTIN_PY_POLL_DIRECTLY` env var.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            poll_directly: std::env::var(POLL_DIRECTLY_ENV)
                .is_ok_and(|value| !value.is_empty() && value != "0"),
            ..Self::default()
        }
    }
}

/// One tile, as the `/{source_ids}/{z}/{x}/{y}` endpoint would send it.
#[derive(Debug, Clone)]
pub struct Tile {
    /// The raw bytes, still compressed when the source stores compressed tiles. Empty when
    /// the source has no data there (the HTTP server answers those with `204 No Content`).
    pub data: TileData,
    /// The `Content-Type`, e.g. `application/x-protobuf` or `image/png`.
    pub content_type: &'static str,
    /// The `Content-Encoding`, e.g. `gzip`, or `None` when the bytes are not compressed.
    pub content_encoding: Option<&'static str>,
    /// The tile's ETag (without quotes); empty for empty tiles.
    pub etag: String,
}

impl Tile {
    /// Whether the source has no data for this tile.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
