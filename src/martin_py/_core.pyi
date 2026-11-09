"""Type stubs for the `martin_py._core` extension module."""

from os import PathLike
from typing import Self

__version__: str

class MartinError(Exception):
    """Base exception for martin_py errors."""

class ConfigError(MartinError):
    """The Martin config could not be read, parsed or resolved into tile sources."""

class UnknownSourceError(MartinError):
    """A requested source id (or alias) is not registered."""

class TileRequestError(MartinError):
    """The tile request cannot be served as asked.

    E.g. a zoom outside the sources' range, sources that cannot be combined, a bad query.
    """

class TileFetchError(MartinError):
    """A source failed while producing the tile."""

class TileServer:
    """Martin's tile sources, loaded once from a Martin config and served in-process."""

    def __new__(
        cls,
        config_path: str | PathLike[str] | None = None,
        *,
        yaml: str | None = None,
        worker_threads: int | None = None,
    ) -> Self:
        """Load and resolve every tile source of a Martin config.

        Pass exactly one of `config_path` or `yaml`.

        Raises:
            ConfigError: the config cannot be read or parsed, or a source fails to resolve.
            ValueError: both or neither of `config_path` and `yaml` were given.
        """

    def list_sources(self) -> list[str]:
        """The ids of all registered tile sources (aliases are not included)."""

    def catalog_json(self) -> str:
        """The tile part of Martin's `/catalog` endpoint, as a JSON string."""

    def get_source_info(self, source_ids: str, tiles_url: str = "") -> str:
        """TileJSON for one or more comma-joined source ids, as a JSON string.

        `source_ids` may combine several sources, e.g. `"a,b"`, the same way the
        `/{source_ids}` HTTP endpoint does. `tiles_url` seeds the advertised `tiles` template;
        `/{z}/{x}/{y}` is appended to it.

        Raises:
            UnknownSourceError: a source id is not registered.
            TileRequestError: the sources cannot be combined.
        """

    def get_tile(
        self,
        source_ids: str,
        z: int,
        x: int,
        y: int,
        query: str | None = None,
    ) -> tuple[bytes, str, str | None]:
        """Fetch one tile's raw, possibly-compressed bytes as they'd be sent over HTTP.

        `source_ids` follows the comma-joined/alias rules of `/{source_ids}/{z}/{x}/{y}`;
        `query` is the URL query string (without `?`) for sources that take parameters.

        Returns `(data, content_type, content_encoding)`; `content_encoding` is `None` when
        the tile is uncompressed, and `data` is empty when the source has no data there.

        Raises:
            ValueError: `z/x/y` is not a valid tile coordinate.
            UnknownSourceError: a source id is not registered.
            TileRequestError: e.g. `z` is outside the sources' zoom range.
            TileFetchError: the source failed.
        """

    async def aget_tile(
        self,
        source_ids: str,
        z: int,
        x: int,
        y: int,
        query: str | None = None,
    ) -> tuple[bytes, str, str | None]:
        """Async `get_tile`, to await inside an asyncio event loop.

        The tile is fetched on the server's own runtime, so the event loop is never blocked.
        Cancelling the coroutine discards the result, but does not stop the fetch.
        """
