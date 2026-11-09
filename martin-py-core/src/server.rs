//! [`TileServer`]: martin's tile sources, loaded once and queried in-process.
use std::future::Future;

use martin::config::file::{Config, parse_config, read_config};
use martin::config::primitives::IdResolver;
use martin::config::primitives::env::{Env as _, OsEnv};
use martin::srv::{DynTileSource, RESERVED_KEYWORDS, TileRequestHeaders, merge_tilejson};
use martin::{StartupResult, TileSourceManager};
use tokio::runtime::{Builder, Runtime};

use crate::error::{MartinPyError, Result};
use crate::types::{ConfigSource, ServerOptions, Tile, TileCoord};

/// Read, finalize and resolve a Martin config into its tile sources.
///
/// This is the startup sequence of the `martin` binary minus the HTTP server. It is free of
/// any runtime so it can be awaited directly, e.g. from a `#[tokio::test]`, but it must run
/// inside a Tokio runtime: sources spawn tasks and open Tokio I/O while resolving.
pub async fn load_tile_manager(source: &ConfigSource) -> StartupResult<TileSourceManager> {
    let mut config: Config = match source {
        ConfigSource::Path(path) => read_config(path, &OsEnv)?,
        ConfigSource::Yaml {
            contents,
            file_name,
        } => parse_config(contents, &OsEnv.as_property_map(), file_name)?,
    };
    config.finalize().await?;
    let resolver = IdResolver::new(RESERVED_KEYWORDS);
    Ok(config.resolve(&resolver).await?.tile_manager)
}

/// Fetch one tile the way `/{source_ids}/{z}/{x}/{y}` does, with default request headers
/// (no `Accept`/`Accept-Encoding` negotiation: the source's own format and encoding).
async fn fetch_tile(
    manager: TileSourceManager,
    source_ids: String,
    xyz: TileCoord,
    query: String,
) -> Result<Tile> {
    let src = DynTileSource::new(
        &manager,
        &source_ids,
        Some(xyz.z()),
        &query,
        TileRequestHeaders::default(),
    )?;
    let tile = src.get_tile_content(xyz).await?;
    Ok(Tile {
        content_type: tile.info.format.content_type(),
        content_encoding: tile.info.encoding.compression(),
        etag: tile.etag.to_string(),
        data: tile.data,
    })
}

/// Martin's tile sources, loaded and resolved once from a Martin config, then served
/// in-process: no network port is opened.
///
/// Owns a multi-threaded Tokio runtime that martin's async source code (file, network and
/// database I/O) runs on. The blocking methods drive it with `block_on`, so they must not be
/// called from inside a Tokio runtime; the `*_async` methods spawn onto it and return a
/// runtime-agnostic future, so they can be awaited from any executor (e.g. a Python event loop).
///
/// `TileServer` is `Send + Sync`; all methods take `&self`.
pub struct TileServer {
    manager: TileSourceManager,
    rt: Runtime,
}

impl TileServer {
    /// Load and resolve every tile source the config declares, like `martin` does at startup.
    pub fn open(source: &ConfigSource, options: &ServerOptions) -> Result<Self> {
        let mut builder = Builder::new_multi_thread();
        builder.enable_all().thread_name("martin-py");
        if let Some(threads) = options.worker_threads {
            if threads == 0 {
                return Err(MartinPyError::InvalidInput(
                    "worker_threads must be >= 1".to_owned(),
                ));
            }
            builder.worker_threads(threads);
        }
        let rt = builder
            .build()
            .map_err(|e| MartinPyError::Runtime(e.to_string()))?;

        let load = load_tile_manager(source);
        let manager = if options.poll_directly {
            // Only enter the runtime, so Tokio I/O, timers and spawned tasks still have one to
            // register with, while the future itself is polled right here.
            let _guard = rt.enter();
            futures::executor::block_on(load)
        } else {
            rt.block_on(load)
        }?;
        Ok(Self { manager, rt })
    }

    /// Wrap an already-resolved manager, e.g. one built by [`load_tile_manager`].
    #[must_use]
    pub fn from_parts(manager: TileSourceManager, rt: Runtime) -> Self {
        Self { manager, rt }
    }

    /// The underlying martin manager, for anything this API does not cover.
    #[must_use]
    pub fn manager(&self) -> &TileSourceManager {
        &self.manager
    }

    /// The ids of all registered tile sources (aliases are not included).
    #[must_use]
    pub fn source_ids(&self) -> Vec<String> {
        self.manager.tile_sources().source_names()
    }

    /// The `/catalog` endpoint's tile section: every source's id and metadata, as JSON.
    pub fn catalog_json(&self) -> Result<String> {
        let catalog = self.manager.tile_sources().get_catalog();
        serde_json::to_string(&catalog).map_err(|e| MartinPyError::Serialize(e.to_string()))
    }

    /// `TileJSON` for one or more comma-joined source ids, as JSON; the `/{source_ids}` endpoint.
    ///
    /// `tiles_url` seeds the advertised `tiles` template: `/{z}/{x}/{y}` is appended to it.
    pub fn tilejson_json(&self, source_ids: &str, tiles_url: &str) -> Result<String> {
        let resolved = self.manager.tile_sources().get_sources(source_ids, None)?;
        let tilejson = merge_tilejson(&resolved.sources, format!("{tiles_url}/{{z}}/{{x}}/{{y}}"));
        serde_json::to_string(&tilejson).map_err(|e| MartinPyError::Serialize(e.to_string()))
    }

    /// Fetch one tile, blocking the calling thread until it is ready.
    ///
    /// `source_ids` follows the comma-joined/alias rules of `/{source_ids}/{z}/{x}/{y}`;
    /// `query` is the URL query string (without `?`) for sources that take parameters.
    pub fn tile(&self, source_ids: &str, xyz: TileCoord, query: &str) -> Result<Tile> {
        self.rt.block_on(fetch_tile(
            self.manager.clone(),
            source_ids.to_owned(),
            xyz,
            query.to_owned(),
        ))
    }

    /// Fetch one tile on the server's runtime; the returned future can be awaited anywhere.
    ///
    /// The work starts immediately, on the server's runtime, and keeps running if the future
    /// is dropped; the result is then discarded.
    ///
    /// martin's tile future cannot be proven `Send` (rustc's higher-ranked lifetime inference
    /// gives up on its closures), so it can't go through `Runtime::spawn`. Instead it is driven
    /// by `Handle::block_on` on one of the runtime's blocking-pool threads: only the closure
    /// has to be `Send`, and its I/O still runs on the runtime's reactor.
    pub fn tile_async(
        &self,
        source_ids: String,
        xyz: TileCoord,
        query: String,
    ) -> impl Future<Output = Result<Tile>> + Send + 'static {
        let rt = self.rt.handle().clone();
        let manager = self.manager.clone();
        let handle = self
            .rt
            .spawn_blocking(move || rt.block_on(fetch_tile(manager, source_ids, xyz, query)));
        async move {
            handle
                .await
                .map_err(|e| MartinPyError::Runtime(format!("tile task failed: {e}")))?
        }
    }

    /// Shut the runtime down without waiting for in-flight tasks to finish.
    ///
    /// Dropping a `TileServer` instead waits for them, which can block; this does not.
    pub fn shutdown_background(self) {
        self.rt.shutdown_background();
    }
}
