#!/usr/bin/env python3
"""P7.27's local fake provider: serves the six families' catalogs straight
from `corpus/providers/` (P7.3's real, recorded evidence) and stands in
for the one thing that corpus was never allowed to hold -- real jar bytes
-- with a locally built fake server jar / fake installer jar. No real
network call happens anywhere in this process.

`crates/msc-infrastructure/src/jar_provider.rs`'s per-family base-URL env
vars (`MSC2_PROVIDER_*_BASE`, added alongside this smoke) are the only
thing that makes a real `msc-agent` process reachable this way -- every
URL path below is otherwise byte-for-byte what the real provider would be
asked for, since only the *host* is overridden, never the path shape.

Vanilla and Paper are the two families whose download URL is data --
read out of the catalog response body by the Rust code, not composed
from a path template -- so those two catalog responses get their `url`
fields rewritten to point back at this same server before being served.
Every other family's download URL is composed by the Rust code itself
from the overridden base, so those catalogs are served byte-for-byte
unmodified.
"""
import argparse
import json
import os
import re
import sys
import zipfile
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from io import BytesIO

ARGS = None


def read_corpus(*parts):
    with open(os.path.join(ARGS.corpus, *parts), "rb") as f:
        return f.read()


def self_base():
    return f"http://127.0.0.1:{ARGS.port}"


def build_installer_jar(properties: dict) -> bytes:
    """Copies the precompiled installer template jar, replacing (or
    adding) its `install-target.properties` resource -- the same "one jar
    per version" shape a real installer download has, without a real
    per-version installer jar to serve."""
    with open(ARGS.installer_template, "rb") as f:
        template_bytes = f.read()
    out = BytesIO()
    props_text = "".join(f"{k}={v}\n" for k, v in properties.items())
    with zipfile.ZipFile(BytesIO(template_bytes), "r") as src, zipfile.ZipFile(
        out, "w", zipfile.ZIP_DEFLATED
    ) as dst:
        for item in src.infolist():
            if item.filename == "install-target.properties":
                continue
            dst.writestr(item, src.read(item.filename))
        dst.writestr("install-target.properties", props_text)
    return out.getvalue()


def is_binary_download(path: str) -> bool:
    return (
        path.startswith("/dl/")
        or path.endswith("-installer.jar")
        or path.endswith("/latest/download")
        or path.endswith("/server/jar")
    )


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args_):
        if ARGS.verbose:
            sys.stderr.write("[fake-provider] " + (fmt % args_) + "\n")

    def _fail_marker_path(self):
        return os.path.join(ARGS.control_dir, "fail_download")

    def _send(self, status: int, body: bytes, content_type: str = "application/octet-stream"):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _send_json(self, obj):
        self._send(200, json.dumps(obj).encode("utf-8"), "application/json")

    def _send_raw(self, path_parts, content_type="application/octet-stream"):
        self._send(200, read_corpus(*path_parts), content_type)

    def do_GET(self):
        path = self.path.split("?", 1)[0]

        if is_binary_download(path) and os.path.exists(self._fail_marker_path()):
            self._send(500, b"injected download failure")
            return

        # ---- Vanilla ----
        if path == "/mc/game/version_manifest_v2.json":
            manifest = json.loads(read_corpus("vanilla", "version-manifest-v2.json"))
            for entry in manifest.get("versions", []):
                entry["url"] = f"{self_base()}/meta/{entry['id']}.json"
            self._send_json(manifest)
            return
        m = re.match(r"^/meta/([^/]+)\.json$", path)
        if m:
            version_json = json.loads(read_corpus("vanilla", "version-26.2.json"))
            version_json["id"] = m.group(1)
            version_json.setdefault("downloads", {}).setdefault("server", {})[
                "url"
            ] = f"{self_base()}/dl/vanilla-server.jar"
            self._send_json(version_json)
            return
        if path == "/dl/vanilla-server.jar":
            with open(ARGS.server_jar, "rb") as f:
                self._send(200, f.read())
            return

        # ---- Purpur ----
        if path == "/v2/purpur":
            self._send_raw(["purpur", "project-purpur.json"], "application/json")
            return
        m = re.match(r"^/v2/purpur/([^/]+)$", path)
        if m:
            self._send_raw(["purpur", "version-1.21.11.json"], "application/json")
            return
        m = re.match(r"^/v2/purpur/([^/]+)/latest/download$", path)
        if m:
            with open(ARGS.server_jar, "rb") as f:
                self._send(200, f.read())
            return

        # ---- Paper ----
        if path == "/v3/projects/paper":
            self._send_raw(["paper", "projects-paper.json"], "application/json")
            return
        m = re.match(r"^/v3/projects/paper/versions/([^/]+)/builds$", path)
        if m:
            builds = json.loads(read_corpus("paper", "builds-1.21.11.json"))
            for build in builds:
                sd = build.get("downloads", {}).get("server:default")
                if sd:
                    sd["url"] = f"{self_base()}/dl/paper-jar"
            self._send_json(builds)
            return
        if path == "/dl/paper-jar":
            with open(ARGS.server_jar, "rb") as f:
                self._send(200, f.read())
            return

        # ---- Fabric ----
        if path == "/v2/versions/game":
            self._send_raw(["fabric", "game.json"], "application/json")
            return
        if path == "/v2/versions/installer":
            self._send_raw(["fabric", "installer.json"], "application/json")
            return
        m = re.match(r"^/v2/versions/loader/([^/]+)$", path)
        if m:
            self._send_raw(["fabric", "loader-1.21.11.json"], "application/json")
            return
        m = re.match(r"^/v2/versions/loader/([^/]+)/([^/]+)/([^/]+)/server/jar$", path)
        if m:
            with open(ARGS.server_jar, "rb") as f:
                self._send(200, f.read())
            return

        # ---- NeoForge ----
        if path == "/releases/net/neoforged/neoforge/maven-metadata.xml":
            self._send_raw(["neoforge", "maven-metadata.xml"], "application/xml")
            return
        m = re.match(
            r"^/releases/net/neoforged/neoforge/([^/]+)/neoforge-\1-installer\.jar$", path
        )
        if m:
            jar_bytes = build_installer_jar(
                {
                    "family": "neoforge",
                    "version": m.group(1),
                    "control_dir": ARGS.control_dir,
                    "install_delay_ms": str(ARGS.install_delay_ms),
                }
            )
            self._send(200, jar_bytes)
            return

        # ---- Forge ----
        if path == "/net/minecraftforge/forge/maven-metadata.xml":
            self._send_raw(["forge", "maven-metadata.xml"], "application/xml")
            return
        if path == "/net/minecraftforge/forge/promotions_slim.json":
            self._send_raw(["forge", "promotions-slim.json"], "application/json")
            return
        m = re.match(
            r"^/net/minecraftforge/forge/([^/]+)/forge-\1-installer\.jar$", path
        )
        if m:
            jar_bytes = build_installer_jar(
                {
                    "family": "forge",
                    "pair": m.group(1),
                    "control_dir": ARGS.control_dir,
                    "install_delay_ms": str(ARGS.install_delay_ms),
                }
            )
            self._send(200, jar_bytes)
            return

        if path == "/__ready__":
            self._send(200, b"ok", "text/plain")
            return

        self._send(404, b"not found")


def main():
    global ARGS
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--corpus", required=True, help="path to corpus/providers")
    parser.add_argument("--server-jar", required=True, help="fake server jar (paper.jar shape)")
    parser.add_argument(
        "--installer-template", required=True, help="fake installer template jar"
    )
    parser.add_argument("--control-dir", required=True)
    parser.add_argument("--install-delay-ms", type=int, default=0)
    parser.add_argument("--verbose", action="store_true")
    ARGS = parser.parse_args()
    os.makedirs(ARGS.control_dir, exist_ok=True)

    server = ThreadingHTTPServer(("127.0.0.1", ARGS.port), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()
