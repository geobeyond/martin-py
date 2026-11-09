"""Shared fixtures for the martin_py test suite.

The data files in `fixtures/` are copied from martin's own `tests/fixtures`, so these tests
cover the same ground as martin's Rust end-to-end tests, but through Python.
"""

from pathlib import Path
from typing import Any

import pytest

import martin_py

FIXTURES = Path(__file__).resolve().parent / "fixtures"


def server_for(config: dict[str, Any]) -> martin_py.TileServer:
    return martin_py.TileServer.from_dict(config)


@pytest.fixture
def geojson_server() -> martin_py.TileServer:
    """A `TileServer` with a single GeoJSON source, `geojson_a`."""
    return server_for(
        {
            "geojson": {
                "sources": {
                    "geojson_a": str(
                        FIXTURES / "geojson" / "feature_collection_1.geojson"
                    ),
                }
            }
        }
    )


@pytest.fixture
def multi_geojson_server() -> martin_py.TileServer:
    """A `TileServer` with two GeoJSON sources, for composite (comma-joined) requests."""
    return server_for(
        {
            "geojson": {
                "sources": {
                    "geojson_a": str(
                        FIXTURES / "geojson" / "feature_collection_1.geojson"
                    ),
                    "geojson_b": str(
                        FIXTURES / "geojson" / "feature_collection_2.geojson"
                    ),
                }
            }
        }
    )


@pytest.fixture
def pmtiles_server() -> martin_py.TileServer:
    """A `TileServer` with a single raster PMTiles source, `pmtiles_a`."""
    return server_for(
        {
            "pmtiles": {
                "sources": {"pmtiles_a": str(FIXTURES / "pmtiles" / "png.pmtiles")}
            }
        }
    )


@pytest.fixture
def bounded_zoom_server() -> martin_py.TileServer:
    """A `TileServer` with a PMTiles source whose zoom range is limited (z0-z3)."""
    path = FIXTURES / "pmtiles" / "stamen_toner__raster_CC-BY+ODbL_z3.pmtiles"
    return server_for({"pmtiles": {"sources": {"stamen_z3": str(path)}}})


@pytest.fixture
def duckdb_server() -> martin_py.TileServer:
    """A `TileServer` with a single DuckDB/GeoParquet source, `polygons`.

    Same fixture and source shape as martin's e2e-tests/tests/duckdb.rs `POLYGONS` config.
    """
    return server_for(
        {
            "duckdb": {
                "sources": [
                    {
                        "geoparquet": str(
                            FIXTURES / "duckdb" / "geoparquet_polygons.parquet"
                        ),
                        "layer_id": "polygons",
                        "geometry_column": "geom",
                        "srid": 4326,
                        "minzoom": 0,
                        "maxzoom": 14,
                    }
                ]
            }
        }
    )
