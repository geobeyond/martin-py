# Configuration

A `TileServer` takes the same config as `martin --config`, so a config file that works with the
`martin` binary works here too. Only the tile sections are used: `pmtiles`, `geojson` (built
with the `processing` feature), `duckdb`, plus `mbtiles`, `postgres` and `cog` when those
features are compiled in. The `srv`, `fonts`, `sprites` and `styles` sections are parsed but
have no effect, because nothing listens on a port.

There are three ways to pass the config:

```python
import martin_py

server = martin_py.TileServer("martin-config.yaml")  # a file
server = martin_py.TileServer.from_yaml(open("cfg.yaml").read())  # YAML text
server = martin_py.TileServer.from_dict(
    {  # a dict, same shape
        "duckdb": {
            "sources": [
                {
                    "geoparquet": "s3://bucket/buildings.parquet",
                    "layer_id": "buildings",
                    "geometry_column": "geom",
                    "srid": 4326,
                }
            ]
        }
    }
)
```

`${VAR}` placeholders are substituted from the environment, as in martin. Relative paths in
YAML text or a dict are resolved against the process's working directory.

## Remote GeoParquet on S3

On the `duckdb_enhancement` branch, DuckDB sources authenticate to `s3://` URLs with martin's
object-store settings. These are the same settings PMTiles sources use: static keys, a
profile, or the AWS credential chain. Credentials are cached and refreshed when they expire,
so a long-lived `TileServer` keeps working with temporary credentials. See the `object_store`
section of martin's config documentation for the keys.

## Load-time errors

All sources are resolved in the constructor. If the config cannot be read or parsed, or any
source fails to resolve, the constructor raises `martin_py.ConfigError`. A `TileServer`
therefore never exists in a half-loaded state.
