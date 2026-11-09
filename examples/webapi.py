"""A minimal tile web API: Martin's tile endpoints, served by FastAPI through martin_py.

Run with:

    MARTIN_CONFIG=martin-config.yaml uv run fastapi dev examples/webapi.py
"""

import os
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from typing import Any

from fastapi import FastAPI, HTTPException, Request, Response

import martin_py

server: martin_py.TileServer


@asynccontextmanager
async def lifespan(_app: FastAPI) -> AsyncGenerator[None]:
    global server
    # load once at startup: martin resolves every source up front
    server = martin_py.TileServer(os.environ.get("MARTIN_CONFIG", "martin-config.yaml"))
    yield


app = FastAPI(lifespan=lifespan)


@app.get("/catalog")
def catalog() -> dict[str, Any]:
    return {"tiles": server.catalog()}


@app.get("/{source_ids}")
def tilejson(source_ids: str, request: Request) -> dict[str, Any]:
    try:
        return server.tilejson(
            source_ids, str(request.url_for("tilejson", source_ids=source_ids))
        )
    except martin_py.UnknownSourceError as e:
        raise HTTPException(404, str(e)) from e


@app.get("/{source_ids}/{z}/{x}/{y}")
async def tile(source_ids: str, z: int, x: int, y: int, request: Request) -> Response:
    try:
        data, content_type, encoding = await server.aget_tile(
            source_ids, z, x, y, request.url.query or None
        )
    except martin_py.UnknownSourceError as e:
        raise HTTPException(404, str(e)) from e
    except (ValueError, martin_py.TileRequestError) as e:
        raise HTTPException(400, str(e)) from e
    if not data:
        return Response(status_code=204)
    headers = {"Content-Encoding": encoding} if encoding else {}
    return Response(data, media_type=content_type, headers=headers)
