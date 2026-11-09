import asyncio

import pytest

import martin_py


def test_aget_tile_matches_get_tile(geojson_server: martin_py.TileServer) -> None:
    expected = geojson_server.get_tile("geojson_a", 0, 0, 0)
    assert asyncio.run(geojson_server.aget_tile("geojson_a", 0, 0, 0)) == expected


def test_atile_returns_a_tile(geojson_server: martin_py.TileServer) -> None:
    tile = asyncio.run(geojson_server.atile("geojson_a", 0, 0, 0))
    assert isinstance(tile, martin_py.Tile)
    assert tile.content_type == "application/x-protobuf"


def test_many_concurrent_requests(multi_geojson_server: martin_py.TileServer) -> None:
    async def main() -> list[tuple[bytes, str, str | None]]:
        return await asyncio.gather(
            *(
                multi_geojson_server.aget_tile(source, 0, 0, 0)
                for source in ["geojson_a", "geojson_b"] * 10
            )
        )

    results = asyncio.run(main())
    assert len(results) == 20
    assert results[0] == results[2]


def test_errors_are_raised_from_the_coroutine(
    geojson_server: martin_py.TileServer,
) -> None:
    with pytest.raises(martin_py.UnknownSourceError):
        asyncio.run(geojson_server.aget_tile("does_not_exist", 0, 0, 0))
    with pytest.raises(ValueError):
        asyncio.run(geojson_server.aget_tile("geojson_a", 1, 5, 0))
