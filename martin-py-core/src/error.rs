//! Error type for martin-py-core.
//!
//! The errors carry plain `String` messages instead of wrapping martin's error types, because:
//!
//! - errors cross thread (and runtime) boundaries, so they must be `Send + 'static`; martin's
//!   `TileError` borrows nothing but is not guaranteed to stay that way
//! - we keep martin's types out of our own public API, which isolates the bindings from
//!   churn in martin's internals (`martin::srv` is not a stable API)
//!
//! The variants are the distinctions a caller can act on; the Python layer maps each one to
//! its own exception class.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MartinPyError {
    /// Caller supplied an out of range or malformed value, e.g. an impossible tile coordinate.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The Martin config could not be read, parsed or resolved into sources.
    #[error("failed to load martin config: {0}")]
    Config(String),

    /// A requested source id (or alias) is not registered.
    #[error("{0}")]
    UnknownSource(String),

    /// The request is well-formed but cannot be served as asked, e.g. a zoom outside the
    /// sources' range, incompatible sources combined, or a malformed query string.
    #[error("{0}")]
    InvalidRequest(String),

    /// A source failed while producing the tile (I/O, database, transcoding, ...).
    #[error("{0}")]
    TileFetch(String),

    /// The Tokio runtime that drives martin could not be created, or a task on it died.
    #[error("tile server runtime error: {0}")]
    Runtime(String),

    /// Serializing a response (TileJSON, catalog) to JSON failed.
    #[error("failed to serialize response: {0}")]
    Serialize(String),
}

pub type Result<T, E = MartinPyError> = std::result::Result<T, E>;

impl From<martin::srv::TileError> for MartinPyError {
    /// Sorts martin's tile errors by who is at fault: the request (bad id, zoom, query, ...)
    /// or the source. This mirrors the 4xx/5xx split the HTTP server makes.
    fn from(err: martin::srv::TileError) -> Self {
        use martin::srv::TileError as E;
        let msg = err.to_string();
        match err {
            E::UnknownSource(_) => Self::UnknownSource(msg),
            E::TooManySources { .. }
            | E::MismatchedSources { .. }
            | E::MismatchedGrids { .. }
            | E::ZoomOutOfRange { .. }
            | E::OutsideGrid { .. }
            | E::UnacceptableFormat(_)
            | E::InvalidQuery(_)
            | E::NoAcceptableEncoding
            | E::UndecodableEncoding(_)
            | E::UnmergeableTiles { .. } => Self::InvalidRequest(msg),
            _ => Self::TileFetch(msg),
        }
    }
}

impl From<martin::StartupError> for MartinPyError {
    fn from(err: martin::StartupError) -> Self {
        Self::Config(err.to_string())
    }
}
