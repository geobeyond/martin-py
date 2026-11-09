//! End-to-end tests of martin-py-core against the fixtures in `../tests/fixtures`.
use std::path::PathBuf;

use martin_py_core::{
    ConfigSource, MartinPyError, ServerOptions, TileCoord, TileServer, load_tile_manager,
};

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(relative)
}

fn geojson_yaml() -> String {
    format!(
        "geojson:\n  sources:\n    geojson_a: {}\n    geojson_b: {}\n",
        fixture("geojson/feature_collection_1.geojson").display(),
        fixture("geojson/feature_collection_2.geojson").display(),
    )
}

fn open(yaml: &str) -> TileServer {
    TileServer::open(&ConfigSource::yaml(yaml), &ServerOptions::default()).unwrap()
}

fn xyz(z: u8, x: u32, y: u32) -> TileCoord {
    TileCoord::new_checked(z, x, y).unwrap()
}

#[tokio::test]
async fn load_tile_manager_resolves_the_declared_sources() {
    let manager = load_tile_manager(&ConfigSource::yaml(geojson_yaml()))
        .await
        .unwrap();
    let mut names = manager.tile_sources().source_names();
    names.sort();
    assert_eq!(names, vec!["geojson_a", "geojson_b"]);
}

#[test]
fn opens_from_a_file_and_from_memory_alike() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    std::fs::write(&path, geojson_yaml()).unwrap();

    let from_file = TileServer::open(&ConfigSource::Path(path), &ServerOptions::default()).unwrap();
    let from_memory = open(&geojson_yaml());

    let mut a = from_file.source_ids();
    let mut b = from_memory.source_ids();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn poll_directly_loads_the_same_sources() {
    let options = ServerOptions {
        poll_directly: true,
        ..ServerOptions::default()
    };
    let server = TileServer::open(&ConfigSource::yaml(geojson_yaml()), &options).unwrap();
    assert_eq!(server.source_ids().len(), 2);
}

#[test]
fn a_missing_config_file_is_a_config_error() {
    let result = TileServer::open(
        &ConfigSource::Path("/no/such/config.yaml".into()),
        &ServerOptions::default(),
    );
    assert!(matches!(result, Err(MartinPyError::Config(_))));
}

#[test]
fn a_source_pointing_at_a_missing_file_is_a_config_error() {
    let result = TileServer::open(
        &ConfigSource::yaml("geojson:\n  sources:\n    gone: /nonexistent/nothing.geojson\n"),
        &ServerOptions::default(),
    );
    assert!(matches!(result, Err(MartinPyError::Config(_))));
}

#[test]
fn zero_worker_threads_is_invalid_input() {
    let options = ServerOptions {
        worker_threads: Some(0),
        ..ServerOptions::default()
    };
    let result = TileServer::open(&ConfigSource::yaml(geojson_yaml()), &options);
    assert!(matches!(result, Err(MartinPyError::InvalidInput(_))));
}

#[test]
fn tilejson_merges_composite_sources() {
    let server = open(&geojson_yaml());
    let tilejson: serde_json::Value = serde_json::from_str(
        &server
            .tilejson_json("geojson_a,geojson_b", "http://localhost/x")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(tilejson["tiles"][0], "http://localhost/x/{z}/{x}/{y}");
    assert_eq!(tilejson["vector_layers"].as_array().unwrap().len(), 2);
}

#[test]
fn catalog_lists_every_source() {
    let server = open(&geojson_yaml());
    let catalog: serde_json::Value = serde_json::from_str(&server.catalog_json().unwrap()).unwrap();
    assert!(catalog.get("geojson_a").is_some());
    assert!(catalog.get("geojson_b").is_some());
}

#[test]
fn tile_returns_uncompressed_mvt() {
    let server = open(&geojson_yaml());
    let tile = server.tile("geojson_a", xyz(0, 0, 0), "").unwrap();
    assert!(!tile.is_empty());
    assert_eq!(tile.content_type, "application/x-protobuf");
    assert_eq!(tile.content_encoding, None);
}

#[test]
fn unknown_sources_are_reported_as_such() {
    let server = open(&geojson_yaml());
    assert!(matches!(
        server.tile("nope", xyz(0, 0, 0), ""),
        Err(MartinPyError::UnknownSource(_))
    ));
    assert!(matches!(
        server.tilejson_json("nope", ""),
        Err(MartinPyError::UnknownSource(_))
    ));
}

#[test]
fn a_zoom_beyond_the_source_range_is_an_invalid_request() {
    let yaml = format!(
        "pmtiles:\n  sources:\n    stamen_z3: {}\n",
        fixture("pmtiles/stamen_toner__raster_CC-BY+ODbL_z3.pmtiles").display()
    );
    let server = open(&yaml);
    assert!(server.tile("stamen_z3", xyz(3, 0, 0), "").is_ok());
    assert!(matches!(
        server.tile("stamen_z3", xyz(4, 0, 0), ""),
        Err(MartinPyError::InvalidRequest(_))
    ));
}

#[test]
fn tile_async_can_be_awaited_from_a_foreign_executor() {
    let server = open(&geojson_yaml());
    let blocking = server.tile("geojson_a", xyz(0, 0, 0), "").unwrap();
    // `futures`' executor knows nothing about Tokio: the future must not need one.
    let awaited = futures::executor::block_on(server.tile_async(
        "geojson_a".to_owned(),
        xyz(0, 0, 0),
        String::new(),
    ))
    .unwrap();
    assert_eq!(blocking.data, awaited.data);
}

#[cfg(feature = "duckdb")]
#[test]
fn duckdb_geoparquet_source_serves_tiles() {
    let yaml = format!(
        "duckdb:\n  sources:\n    - geoparquet: {}\n      layer_id: polygons\n      geometry_column: geom\n      srid: 4326\n      minzoom: 0\n      maxzoom: 14\n",
        fixture("duckdb/geoparquet_polygons.parquet").display()
    );
    let server = open(&yaml);
    assert_eq!(server.source_ids(), vec!["polygons"]);
    let tile = server.tile("polygons", xyz(1, 0, 0), "").unwrap();
    assert!(!tile.is_empty());
    assert_eq!(tile.content_type, "application/x-protobuf");
}
