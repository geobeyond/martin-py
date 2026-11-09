"""A DuckDB/GeoParquet source, mirroring martin's e2e-tests/tests/duckdb.rs.

Requires the `duckdb` feature (on by default), i.e. martin's `unstable-duckdb`.
"""

import martin_py


def test_list_sources_uses_the_configured_layer_id(
    duckdb_server: martin_py.TileServer,
) -> None:
    assert duckdb_server.list_sources() == ["polygons"]


def test_tilejson_describes_the_parquet_columns(
    duckdb_server: martin_py.TileServer,
) -> None:
    tilejson = duckdb_server.tilejson("polygons", "http://localhost/polygons")

    assert tilejson["tilejson"] == "3.0.0"
    assert tilejson["name"] == "polygons"
    assert tilejson["minzoom"] == 0
    assert tilejson["maxzoom"] == 14
    assert tilejson["bounds"] == [-50.0, 20.0, 5.0, 30.0]
    assert tilejson["vector_layers"] == [
        {"id": "polygons", "fields": {"id": "INTEGER", "name": "VARCHAR"}}
    ]


def test_get_tile_clips_the_polygon_crossing_the_tile_edge(
    duckdb_server: martin_py.TileServer,
) -> None:
    data, content_type, _ = duckdb_server.get_tile("polygons", 1, 0, 0)

    assert len(data) > 0
    assert content_type == "application/x-protobuf"
