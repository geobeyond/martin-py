"""`martin_py.TileServer`, covering the same ground as Martin's `/catalog`, `/{source_ids}`
and `/{source_ids}/{z}/{x}/{y}` HTTP endpoints, but in-process.
"""

import json
import threading
from pathlib import Path
from typing import Any

import pytest

import martin_py

FIXTURES = Path(__file__).resolve().parent / "fixtures"


class TestConstruction:
    def test_from_config_file(self, tmp_path: Path) -> None:
        config = tmp_path / "config.yaml"
        geojson = FIXTURES / "geojson" / "feature_collection_1.geojson"
        config.write_text(f"geojson:\n  sources:\n    geojson_a: {geojson}\n")

        assert martin_py.TileServer(config).list_sources() == ["geojson_a"]
        # a plain str path works too
        assert martin_py.TileServer(str(config)).list_sources() == ["geojson_a"]

    def test_from_yaml(self) -> None:
        geojson = FIXTURES / "geojson" / "feature_collection_1.geojson"
        server = martin_py.TileServer.from_yaml(
            f"geojson:\n  sources:\n    a: {geojson}\n"
        )
        assert server.list_sources() == ["a"]

    def test_is_the_python_subclass(self, geojson_server: martin_py.TileServer) -> None:
        assert type(geojson_server) is martin_py.TileServer

    def test_missing_config_file_is_a_config_error(self) -> None:
        with pytest.raises(martin_py.ConfigError):
            martin_py.TileServer("/no/such/config.yaml")

    def test_config_with_no_sources_is_a_config_error(self, tmp_path: Path) -> None:
        empty_config = tmp_path / "empty.yaml"
        empty_config.write_text("{}\n")
        with pytest.raises(martin_py.ConfigError):
            martin_py.TileServer(empty_config)

    def test_invalid_yaml_is_a_config_error(self) -> None:
        with pytest.raises(martin_py.ConfigError):
            martin_py.TileServer.from_yaml("geojson: [unclosed")

    @pytest.mark.parametrize(
        "kwargs", [{}, {"config_path": "x.yaml", "yaml": "{}"}], ids=["neither", "both"]
    )
    def test_exactly_one_config_is_required(self, kwargs: dict[str, Any]) -> None:
        with pytest.raises(ValueError):
            martin_py.TileServer(**kwargs)

    def test_errors_share_a_base_class(self) -> None:
        for exc in (
            martin_py.ConfigError,
            martin_py.UnknownSourceError,
            martin_py.TileRequestError,
            martin_py.TileFetchError,
        ):
            assert issubclass(exc, martin_py.MartinError)


class TestGeoJsonSource:
    def test_list_sources(self, geojson_server: martin_py.TileServer) -> None:
        assert geojson_server.list_sources() == ["geojson_a"]

    def test_catalog(self, geojson_server: martin_py.TileServer) -> None:
        assert set(geojson_server.catalog()) == {"geojson_a"}

    def test_tilejson(self, geojson_server: martin_py.TileServer) -> None:
        tilejson = geojson_server.tilejson("geojson_a", "http://localhost/geojson_a")

        assert tilejson["tilejson"] == "3.0.0"
        assert tilejson["tiles"] == ["http://localhost/geojson_a/{z}/{x}/{y}"]
        assert "vector_layers" in tilejson

    def test_get_source_info_is_the_same_tilejson_as_text(
        self, geojson_server: martin_py.TileServer
    ) -> None:
        text = geojson_server.get_source_info("geojson_a", "http://localhost/geojson_a")
        assert json.loads(text) == geojson_server.tilejson(
            "geojson_a", "http://localhost/geojson_a"
        )

    def test_unknown_source(self, geojson_server: martin_py.TileServer) -> None:
        with pytest.raises(martin_py.UnknownSourceError):
            geojson_server.tilejson("does_not_exist")
        with pytest.raises(martin_py.UnknownSourceError):
            geojson_server.get_tile("does_not_exist", 0, 0, 0)

    def test_get_tile_returns_mvt_bytes(
        self, geojson_server: martin_py.TileServer
    ) -> None:
        data, content_type, content_encoding = geojson_server.get_tile(
            "geojson_a", 0, 0, 0
        )

        assert isinstance(data, bytes)
        assert len(data) > 0
        assert content_type == "application/x-protobuf"
        # a freshly computed in-process tile isn't compressed
        assert content_encoding is None

    def test_tile_named_tuple(self, geojson_server: martin_py.TileServer) -> None:
        tile = geojson_server.tile("geojson_a", 0, 0, 0)
        assert isinstance(tile, martin_py.Tile)
        assert tuple(tile) == geojson_server.get_tile("geojson_a", 0, 0, 0)

    def test_invalid_coordinate_is_a_value_error(
        self, geojson_server: martin_py.TileServer
    ) -> None:
        with pytest.raises(ValueError):
            geojson_server.get_tile("geojson_a", 1, 5, 0)

    def test_concurrent_calls_from_threads(
        self, geojson_server: martin_py.TileServer
    ) -> None:
        expected = geojson_server.get_tile("geojson_a", 0, 0, 0)
        results: list[tuple[bytes, str, str | None]] = []

        def fetch() -> None:
            results.append(geojson_server.get_tile("geojson_a", 0, 0, 0))

        threads = [threading.Thread(target=fetch) for _ in range(8)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        assert results == [expected] * 8


class TestCompositeSources:
    """`source_ids` may comma-join several sources into one composite request."""

    def test_list_sources_includes_both(
        self, multi_geojson_server: martin_py.TileServer
    ) -> None:
        assert sorted(multi_geojson_server.list_sources()) == ["geojson_a", "geojson_b"]

    def test_tilejson_merges_both_sources(
        self, multi_geojson_server: martin_py.TileServer
    ) -> None:
        tilejson = multi_geojson_server.tilejson("geojson_a,geojson_b")
        layer_ids = {layer["id"] for layer in tilejson["vector_layers"]}
        assert layer_ids == {"geojson_a", "geojson_b"}

    def test_get_tile_combines_both_sources(
        self, multi_geojson_server: martin_py.TileServer
    ) -> None:
        data, content_type, _ = multi_geojson_server.get_tile(
            "geojson_a,geojson_b", 0, 0, 0
        )
        assert len(data) > 0
        assert content_type == "application/x-protobuf"


class TestPmtilesSource:
    def test_get_tile_returns_png_bytes(
        self, pmtiles_server: martin_py.TileServer
    ) -> None:
        data, content_type, _ = pmtiles_server.get_tile("pmtiles_a", 0, 0, 0)

        assert content_type == "image/png"
        assert data[:8] == b"\x89PNG\r\n\x1a\n"


class TestZoomBounds:
    def test_within_supported_zoom_range(
        self, bounded_zoom_server: martin_py.TileServer
    ) -> None:
        maxzoom = bounded_zoom_server.tilejson("stamen_z3")["maxzoom"]
        data, _, _ = bounded_zoom_server.get_tile("stamen_z3", maxzoom, 0, 0)
        assert len(data) > 0

    def test_beyond_supported_zoom_range(
        self, bounded_zoom_server: martin_py.TileServer
    ) -> None:
        maxzoom = bounded_zoom_server.tilejson("stamen_z3")["maxzoom"]
        with pytest.raises(martin_py.TileRequestError):
            bounded_zoom_server.get_tile("stamen_z3", maxzoom + 1, 0, 0)
