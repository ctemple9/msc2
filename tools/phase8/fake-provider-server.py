#!/usr/bin/env python3
"""Local Modrinth-shaped provider for the Phase 8 synthetic smoke.

It serves only deterministic metadata and a tiny JAR-shaped payload over
loopback.  The smoke never contacts a public provider.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


ARGS: argparse.Namespace


def response(handler: BaseHTTPRequestHandler, status: int, body: bytes, content_type: str) -> None:
    handler.send_response(status)
    handler.send_header("Content-Type", content_type)
    handler.send_header("Content-Length", str(len(body)))
    handler.end_headers()
    handler.wfile.write(body)


def version(base: str) -> dict[str, object]:
    payload = b"PK\x03\x04msc2-phase8-synthetic-addon"
    return {
        "id": "synthetic-version-2",
        "project_id": "synthetic-project",
        "name": "Synthetic Addon 2.0",
        "version_number": "2.0.0",
        "game_versions": ["1.20.1"],
        "loaders": ["fabric"],
        "dependencies": [],
        "files": [{
            "url": f"{base}/downloads/synthetic-addon-2.0.jar",
            "filename": "synthetic-addon-2.0.jar",
            "primary": True,
            "hashes": {"sha1": hashlib.sha1(payload).hexdigest()},
        }],
    }


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_: object) -> None:
        pass

    def do_GET(self) -> None:  # noqa: N802
        base = f"http://127.0.0.1:{ARGS.port}"
        path = self.path.split("?", 1)[0]
        if path == "/__ready__":
            response(self, 200, b"ready\n", "text/plain")
        elif path == "/v2/search":
            body = {"hits": [{"project_id": "synthetic-project", "slug": "synthetic-addon", "title": "Synthetic Addon"}], "offset": 0, "limit": 20, "total_hits": 1}
            response(self, 200, json.dumps(body).encode(), "application/json")
        elif path in {"/v2/project/synthetic-project", "/v2/project/synthetic-addon"}:
            response(self, 200, json.dumps({"id": "synthetic-project", "slug": "synthetic-addon", "title": "Synthetic Addon"}).encode(), "application/json")
        elif path in {"/v2/project/synthetic-project/version", "/v2/project/synthetic-addon/version"}:
            response(self, 200, json.dumps([version(base)]).encode(), "application/json")
        elif path == "/downloads/synthetic-addon-2.0.jar":
            response(self, 200, b"PK\x03\x04msc2-phase8-synthetic-addon", "application/java-archive")
        else:
            response(self, 404, b"not found\n", "text/plain")


def main() -> None:
    global ARGS
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    ARGS = parser.parse_args()
    ThreadingHTTPServer(("127.0.0.1", ARGS.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
