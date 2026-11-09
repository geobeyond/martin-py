//! PyO3 bindings for martin-py-core.
//!
//! This crate is deliberately thin: it converts Python arguments into martin-py-core types,
//! releases the GIL around anything slow, and converts results and errors back. All real logic
//! is in martin-py-core.

use std::mem::ManuallyDrop;
use std::path::PathBuf;

use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use martin_py_core::{ConfigSource, MartinPyError, ServerOptions, Tile, TileCoord};

// We use the name `martin_py` (and not `martin_py._core`) for the exceptions because users
// import them from the `martin_py` python package
create_exception!(
    martin_py,
    MartinError,
    PyException,
    "Base exception for martin_py errors"
);
create_exception!(
    martin_py,
    ConfigError,
    MartinError,
    "The Martin config could not be read, parsed or resolved into tile sources."
);
create_exception!(
    martin_py,
    UnknownSourceError,
    MartinError,
    "A requested source id (or alias) is not registered."
);
create_exception!(
    martin_py,
    TileRequestError,
    MartinError,
    "The tile request cannot be served as asked (zoom out of range, incompatible sources, bad query...)."
);
create_exception!(
    martin_py,
    TileFetchError,
    MartinError,
    "A source failed while producing the tile."
);

fn to_py_err(err: MartinPyError) -> PyErr {
    use MartinPyError as E;
    let msg = err.to_string();
    match err {
        E::InvalidInput(_) => PyValueError::new_err(msg),
        E::Config(_) => ConfigError::new_err(msg),
        E::UnknownSource(_) => UnknownSourceError::new_err(msg),
        E::InvalidRequest(_) => TileRequestError::new_err(msg),
        E::TileFetch(_) => TileFetchError::new_err(msg),
        _ => MartinError::new_err(msg),
    }
}

fn tile_coord(z: u8, x: u32, y: u32) -> PyResult<TileCoord> {
    TileCoord::new_checked(z, x, y)
        .ok_or_else(|| PyValueError::new_err(format!("invalid tile coordinate {z}/{x}/{y}")))
}

/// `(data, content_type, content_encoding)`, the shape `get_tile` returns.
type TileTuple = (Py<PyBytes>, &'static str, Option<&'static str>);

fn tile_to_py(py: Python<'_>, tile: &Tile) -> TileTuple {
    (
        PyBytes::new(py, &tile.data).unbind(),
        tile.content_type,
        tile.content_encoding,
    )
}

/// Martin's tile sources, loaded once from a Martin config and served in-process.
///
/// `frozen`: Python only ever gets an immutable reference, so PyO3 can skip runtime borrow
/// tracking and the object can be shared freely across Python threads. This works because
/// martin_py_core::TileServer is `Send + Sync` and only needs `&self`.
///
/// `subclass`: the `martin_py.TileServer` Python class extends this one with conveniences
/// (dict-returning helpers, alternative constructors).
#[pyclass(frozen, subclass, module = "martin_py._core")]
struct TileServer {
    /// Dropped by hand, see `Drop`
    inner: ManuallyDrop<martin_py_core::TileServer>,
}

impl Drop for TileServer {
    /// Dropping the server shuts down its Tokio runtime, which waits for in-flight tasks.
    /// Release the GIL while that happens, so other Python threads are not stalled by it.
    fn drop(&mut self) {
        // SAFETY: `inner` is not used again after being taken here
        let inner = unsafe { ManuallyDrop::take(&mut self.inner) };
        Python::attach(|py| py.detach(move || drop(inner)));
    }
}

#[pymethods]
impl TileServer {
    /// Load and resolve every tile source of a Martin config: pass exactly one of `config_path`
    /// (a file) or `yaml` (the config's text; JSON is valid YAML).
    #[new]
    #[pyo3(signature = (config_path=None, *, yaml=None, worker_threads=None))]
    fn new(
        py: Python<'_>,
        config_path: Option<PathBuf>,
        yaml: Option<String>,
        worker_threads: Option<usize>,
    ) -> PyResult<Self> {
        let source = match (config_path, yaml) {
            (Some(path), None) => ConfigSource::Path(path),
            (None, Some(text)) => ConfigSource::yaml(text),
            _ => {
                return Err(PyValueError::new_err(
                    "pass exactly one of config_path or yaml",
                ));
            }
        };
        let options = ServerOptions {
            worker_threads,
            ..ServerOptions::from_env()
        };
        // Loading may open files, databases and remote object stores: don't hold the GIL.
        let inner = py
            .detach(|| martin_py_core::TileServer::open(&source, &options))
            .map_err(to_py_err)?;
        Ok(Self {
            inner: ManuallyDrop::new(inner),
        })
    }

    /// The ids of all registered tile sources (aliases are not included).
    fn list_sources(&self) -> Vec<String> {
        self.inner.source_ids()
    }

    /// The tile part of the `/catalog` endpoint, as a JSON string.
    fn catalog_json(&self) -> PyResult<String> {
        self.inner.catalog_json().map_err(to_py_err)
    }

    /// TileJSON for one or more comma-joined source ids, as a JSON string.
    #[pyo3(signature = (source_ids, tiles_url=""))]
    fn get_source_info(
        &self,
        py: Python<'_>,
        source_ids: &str,
        tiles_url: &str,
    ) -> PyResult<String> {
        py.detach(|| self.inner.tilejson_json(source_ids, tiles_url))
            .map_err(to_py_err)
    }

    /// Fetch one tile's raw, possibly-compressed bytes as they'd be sent over HTTP.
    #[pyo3(signature = (source_ids, z, x, y, query=None))]
    fn get_tile(
        &self,
        py: Python<'_>,
        source_ids: &str,
        z: u8,
        x: u32,
        y: u32,
        query: Option<&str>,
    ) -> PyResult<TileTuple> {
        let xyz = tile_coord(z, x, y)?;
        let tile = py
            .detach(|| self.inner.tile(source_ids, xyz, query.unwrap_or_default()))
            .map_err(to_py_err)?;
        Ok(tile_to_py(py, &tile))
    }

    /// Async `get_tile`: the tile is fetched on the server's own runtime, and this coroutine
    /// only awaits the result, so the event loop is never blocked.
    #[pyo3(signature = (source_ids, z, x, y, query=None))]
    async fn aget_tile(
        &self,
        source_ids: String,
        z: u8,
        x: u32,
        y: u32,
        query: Option<String>,
    ) -> PyResult<TileTuple> {
        let xyz = tile_coord(z, x, y)?;
        let tile = self
            .inner
            .tile_async(source_ids, xyz, query.unwrap_or_default())
            .await
            .map_err(to_py_err)?;
        Ok(Python::attach(|py| tile_to_py(py, &tile)))
    }
}

/// Install a stderr `tracing` subscriber so martin's `debug!`/`info!` events are visible.
///
/// Filtered by `RUST_LOG` (e.g. `martin=debug,martin_core=debug`), defaulting to `info`.
/// `RUST_LOG` is read once, when Python first imports `martin_py`. Uses `try_init` so a
/// subscriber installed elsewhere (re-import, embedding host, tests) is left alone.
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_tracing();
    let py = m.py();
    m.add_class::<TileServer>()?;
    m.add("MartinError", py.get_type::<MartinError>())?;
    m.add("ConfigError", py.get_type::<ConfigError>())?;
    m.add("UnknownSourceError", py.get_type::<UnknownSourceError>())?;
    m.add("TileRequestError", py.get_type::<TileRequestError>())?;
    m.add("TileFetchError", py.get_type::<TileFetchError>())?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
