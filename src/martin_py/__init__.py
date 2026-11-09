"""Serve Martin tile sources in-process: Martin's tile endpoints, without HTTP."""

import json as _json
from os import PathLike
from typing import Any, NamedTuple

from . import _core
from ._core import (
    ConfigError,
    MartinError,
    TileFetchError,
    TileRequestError,
    UnknownSourceError,
    __version__,
)

__all__ = [
    "ConfigError",
    "MartinError",
    "Tile",
    "TileFetchError",
    "TileRequestError",
    "TileServer",
    "UnknownSourceError",
    "__version__",
]


class Tile(NamedTuple):
    """One tile, as Martin's `/{source_ids}/{z}/{x}/{y}` endpoint would send it.

    A plain tuple, so `data, content_type, content_encoding = tile` keeps working.
    """

    data: bytes
    """The raw bytes, still compressed when the source stores compressed tiles. Empty when
    the source has no data there (Martin answers those with `204 No Content`)."""
    content_type: str
    """The `Content-Type`, e.g. `"application/x-protobuf"` or `"image/png"`."""
    content_encoding: str | None
    """The `Content-Encoding`, e.g. `"gzip"`, or `None` when the bytes are not compressed."""


class TileServer(_core.TileServer):
    """Martin's tile sources, loaded once from a Martin config and served in-process.

    All sources are resolved up front, just like the `martin` server does at startup; tile
    requests are then answered directly, without opening any network port. A `TileServer` is
    safe to share between threads, and every slow call releases the GIL.

    Args:
        config_path: a Martin config file, the same one `martin --config` takes.
        yaml: the config's text instead of a file. JSON is valid YAML, see also `from_dict`.
        worker_threads: threads of the Tokio runtime the sources run on; one per core by default.

    Raises:
        ConfigError: the config cannot be read or parsed, or a source fails to resolve.
    """

    def __init__(
        self,
        config_path: str | PathLike[str] | None = None,
        *,
        yaml: str | None = None,
        worker_threads: int | None = None,
    ) -> None:
        # the work happens in `_core.TileServer.__new__`; this only documents the signature
        pass

    @classmethod
    def from_yaml(cls, text: str, *, worker_threads: int | None = None) -> "TileServer":
        """Load the Martin config given as YAML text.

        Relative paths in it are resolved against the current working directory.
        """
        return cls(yaml=text, worker_threads=worker_threads)

    @classmethod
    def from_dict(
        cls, config: dict[str, Any], *, worker_threads: int | None = None
    ) -> "TileServer":
        """Load a Martin config given as a dict, with the same shape as the YAML file."""
        return cls(yaml=_json.dumps(config), worker_threads=worker_threads)

    def catalog(self) -> dict[str, Any]:
        """The tile sources of Martin's `/catalog` endpoint: source id -> metadata."""
        return _json.loads(self.catalog_json())

    def tilejson(self, source_ids: str, tiles_url: str = "") -> dict[str, Any]:
        """TileJSON for one or more comma-joined source ids, as `/{source_ids}` returns it.

        `tiles_url` seeds the advertised `tiles` template; `/{z}/{x}/{y}` is appended to it.

        Raises:
            UnknownSourceError: a source id is not registered.
            TileRequestError: the sources cannot be combined.
        """
        return _json.loads(self.get_source_info(source_ids, tiles_url))

    def tile(
        self, source_ids: str, z: int, x: int, y: int, query: str | None = None
    ) -> Tile:
        """`get_tile`, returning a `Tile` named tuple."""
        return Tile(*self.get_tile(source_ids, z, x, y, query))

    async def atile(
        self, source_ids: str, z: int, x: int, y: int, query: str | None = None
    ) -> Tile:
        """`aget_tile`, returning a `Tile` named tuple."""
        return Tile(*await self.aget_tile(source_ids, z, x, y, query))
