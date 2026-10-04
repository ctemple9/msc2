#!/usr/bin/env python3
"""Prepare/build the isolated client, validate exports, and run the owner proof.

Only `run` launches Minecraft. `prepare`, `build` and syntax/build checks do not.
No existing launcher instances, mods or worlds are written by this tool.
"""
from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import threading
import webbrowser
import zlib
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parent
PINS = json.loads((ROOT / "pins.json").read_text())
MAX_EXPORT = 256 * 1024 * 1024
NAME = re.compile(r"capture-[0-9a-f-]{36}\Z")
FILES = re.compile(r"(?:mesh-\d+\.json|texture-\d+\.png|minecraft-frame\.png)\Z")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def atomic_json(path: Path, value: dict) -> None:
    temp = path.with_suffix(".tmp")
    with temp.open("x") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.flush()
        os.fsync(stream.fileno())
    temp.replace(path)


def regular(path: Path, limit: int) -> bytes:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise ValueError(f"Refused non-regular or oversized artifact: {path.name}")
    with path.open("rb") as stream:
        value = stream.read(limit + 1)
    if len(value) > limit:
        raise ValueError(f"Artifact exceeds limit: {path.name}")
    return value


def prepare(work: Path, source: Path | None) -> None:
    for path in [work, work / "client", work / "mods", work / "client/mods", work / "exports", work / "evidence"]:
        if path.is_symlink() or (path.exists() and not path.is_dir()):
            raise ValueError("Symlink or non-directory proof path refused")
    if (work / "client").exists():
        count = 0
        for path in (work / "client").rglob("*"):
            count += 1
            if path.is_symlink() or count > 100000:
                raise ValueError("Linked or oversized isolated client tree refused")
    # Refuse to repurpose an arbitrary existing folder as a launcher game directory.
    marker = work / ".msc-proof-workspace"
    if marker.is_symlink() or (work.exists() and not marker.is_file() and any(work.iterdir())):
        raise ValueError("Workspace is not an empty or marked MSC proof workspace")
    work.mkdir(parents=True, exist_ok=True)
    marker.write_text("MSC isolated supplemental mesh proof 1\n")
    client = work / "client"
    client.mkdir(exist_ok=True)
    (client / ".msc-proof-client").write_text("MSC isolated proof client 1\n")
    mods = work / "mods"
    mods.mkdir(exist_ok=True)
    known = {p["file"] for p in PINS["mods"]}
    if any(p.is_symlink() or not p.is_file() or p.name not in known for p in mods.iterdir()):
        raise ValueError("Unrecognized input in isolated mods folder")
    candidates = [source] if source else [
        Path.home() / ".local/share/PrismLauncher/instances/ATM10/minecraft/mods",
        Path.home() / ".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/ATM10/minecraft/mods",
    ]
    client_mods = client / "mods"
    client_mods.mkdir(exist_ok=True)
    if any(p.is_symlink() or not p.is_file() or p.name not in known for p in client_mods.iterdir()):
        raise ValueError("Unexpected mod in the isolated proof client")
    receipts = []
    for pin in PINS["mods"]:
        dest = mods / pin["file"]
        src = dest if dest.exists() else next((p / pin["file"] for p in candidates if p and (p / pin["file"]).is_file()), None)
        if src is None:
            raise ValueError(f"Needs exact {pin['file']} (SHA-256 {pin['sha256']}); supply --source-mods DIRECTORY. Do not substitute another release.")
        raw = regular(src, pin["bytes"])
        if len(raw) != pin["bytes"] or digest(raw) != pin["sha256"]:
            raise ValueError(f"Exact fixture hash/size mismatch: {pin['file']}")
        with ZipFile(src) as archive:
            model = archive.read("assets/supplementaries/models/block/barnacles.json") if pin["id"] == "supplementaries" else None
            if model and json.loads(model)["loader"] != "supplementaries:random_rotation":
                raise ValueError("Fixture is not the pinned real custom-loader model")
        if src != dest:
            temp = mods / (pin["file"] + ".tmp")
            temp.write_bytes(raw)
            temp.replace(dest)
        installed = client_mods / pin["file"]
        if installed.exists() and digest(regular(installed, pin["bytes"])) != pin["sha256"]:
            raise ValueError("Modified isolated client input; remove the changed proof file before preparing again")
        if not installed.exists():
            installed.write_bytes(raw)
        receipts.append({**pin, "evidence": "local_hashed"})
    sources = sorted(p for p in ROOT.rglob("*") if p.is_file() and not any(part in {"build", ".gradle", "node_modules", "__pycache__"} for part in p.relative_to(ROOT).parts))
    source_hash = digest("\n".join(f"{p.relative_to(ROOT)}:{digest(p.read_bytes())}" for p in sources).encode())
    receipt = {"minecraft": PINS["minecraft"], "neoforge": PINS["neoforge"], "mods": receipts,
               "prototypeSourceSha256": source_hash, "gradle": PINS["gradle"],
               "gameArtifacts": [], "visualAcceptance": "pending"}
    # Build cache artifacts supplement receipts when present; no launcher accounts are read.
    artifacts = Path.home() / ".gradle/caches/neoformruntime/artifacts"
    client_jar = artifacts / "minecraft_1.21.1_client.jar"
    neo_root = Path.home() / ".gradle/caches/modules-2/files-2.1/net.neoforged/neoforge/21.1.251"
    for path in ([client_jar] if client_jar.is_file() else []) + sorted(neo_root.glob("*/neoforge-21.1.251-universal.jar")):
        raw = regular(path, 64 * 1024 * 1024)
        receipt["gameArtifacts"].append({"file": path.name, "sha256": digest(raw), "bytes": len(raw), "evidence": "local_hashed"})
    atomic_json(work / "inputs.json", receipt)
    options = client / "options.txt"
    if not options.exists():
        options.write_text("renderDistance:4\nsimulationDistance:4\nguiScale:2\nfov:0.0\n")
    print(f"Prepared pinned input bytes in {work}; no game launched.")


def gradle(explicit: str | None) -> str:
    paths = [Path(explicit)] if explicit else []
    executable = shutil.which("gradle")
    if executable:
        paths.append(Path(executable))
    paths.extend(sorted((Path.home() / ".gradle/wrapper/dists/gradle-9.2.1-bin").glob("*/gradle-9.2.1/bin/gradle")))
    for path in paths:
        if path.is_file():
            result = subprocess.run([str(path), "--version"], text=True, capture_output=True, check=True)
            if re.search(r"^Gradle 9\.2\.1\s*$", result.stdout, re.M):
                return str(path)
    raise ValueError("Gradle 9.2.1 is required; pass --gradle /path/to/gradle-9.2.1/bin/gradle")


def build(work: Path, executable: str) -> None:
    subprocess.run([executable, "--no-daemon", "-p", str(ROOT), f"-PproofWorkspace={work}", "jar", "prepareClientRun", "writeProofLaunchReceipt"], check=True)
    prepare(work, None)
    inputs = json.loads(regular(work / "inputs.json", 4*1024*1024))
    if any(not any(all(artifact.get(k) == pin[k] for k in ["file", "bytes", "sha256"]) for artifact in inputs["gameArtifacts"]) for pin in PINS["gameArtifacts"]):
        raise ValueError("Pinned game/loader artifact receipt missing or mismatched; no game launch")
    jar = ROOT / "build/libs/msc-client-mesh-proof-0.1.0.jar"
    raw = regular(jar, 4 * 1024 * 1024)
    atomic_json(work / "build-receipt.json", {"prototypeJarSha256": digest(raw), "bytes": len(raw), "pins": PINS,
                                            "gradleExecutableSha256": digest(Path(executable).read_bytes()),
                                            "launch": json.loads(regular(work / "launch-receipt.json", 100000)),
                                            "checks": "jar and prepareClientRun; no tests or game execution"})


def png(raw: bytes, maximum: int = 8192) -> tuple[int, int]:
    if len(raw) < 33 or raw[:8] != b"\x89PNG\r\n\x1a\n" or raw[12:16] != b"IHDR":
        raise ValueError("Invalid PNG header")
    w, h = struct.unpack(">II", raw[16:24])
    if not 0 < w <= maximum or not 0 < h <= maximum or w * h * 4 > MAX_EXPORT:
        raise ValueError("PNG decoded dimensions exceed limits")
    if raw[24] != 8 or raw[25] not in {2, 6} or raw[26:29] != bytes(3):
        raise ValueError("Unsupported exported PNG encoding")
    offset = 8
    compressed = bytearray()
    ended = False
    while offset < len(raw):
        if offset + 12 > len(raw):
            raise ValueError("Truncated PNG chunk")
        size = struct.unpack(">I", raw[offset:offset+4])[0]
        end = offset + 12 + size
        if end > len(raw):
            raise ValueError("PNG chunk length exceeds file")
        kind = raw[offset+4:offset+8]
        contents = raw[offset+8:offset+8+size]
        crc = struct.unpack(">I", raw[offset+8+size:end])[0]
        if binascii.crc32(kind + contents) & 0xffffffff != crc:
            raise ValueError("PNG chunk checksum mismatch")
        if kind == b"IDAT":
            compressed.extend(contents)
        if kind == b"IEND":
            if size or end != len(raw):
                raise ValueError("Invalid PNG end")
            ended = True
            break
        offset = end
    if not ended or not compressed:
        raise ValueError("PNG image data missing")
    expected = h * (w * (4 if raw[25] == 6 else 3) + 1)
    decoder = zlib.decompressobj()
    pixels = decoder.decompress(compressed, expected + 1)
    if len(pixels) != expected or not decoder.eof or decoder.unused_data or decoder.unconsumed_tail:
        raise ValueError("PNG decompression size/stream mismatch")
    stride = expected // h
    if any(pixels[row*stride] > 4 for row in range(h)):
        raise ValueError("PNG filter encoding refused")
    return w, h


def validate(directory: Path, expected: dict | None = None) -> tuple[dict, dict]:
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError("Invalid capture directory")
    raw = regular(directory / "capture.json", 4 * 1024 * 1024)
    manifest = json.loads(raw)
    if manifest["format"] != "msc-supplemental-mesh-proof-1" or manifest["appearance"] != "saved_frame":
        raise ValueError("Unknown format or animation appearance")
    if manifest["minecraft"] != PINS["minecraft"] or manifest["neoforge"] != PINS["neoforge"]:
        raise ValueError("Client/loader version mismatch")
    for key in ["snapshotSha256", "contextSha256"]:
        if not re.fullmatch("[0-9a-f]{64}", manifest[key]):
            raise ValueError(f"Invalid binding: {key}")
        if expected and manifest[key] != expected[key]:
            raise ValueError(f"Binding mismatch: {key}")
    if manifest["dimension"] != "minecraft:overworld" or manifest["partialTick"] != 0:
        raise ValueError("Unsupported proof dimension/frame")
    lights = manifest["directionalLights"]
    if len(lights) != 2 or any(len(light) != 3 or any(type(v) not in {int,float} or not math.isfinite(v) or abs(v) > 2 for v in light) for light in lights):
        raise ValueError("Invalid captured directional lighting")
    if not re.fullmatch("[0-9a-f]{64}", manifest["resourceContextSha256"]):
        raise ValueError("Resource context missing")
    mods = manifest["inputs"]["mods"]
    if len(mods) != len(PINS["mods"]) or any(any(mod.get(k) != pin[k] for k in ["id", "version", "bytes", "sha256", "file"])
                                                 for mod, pin in zip(mods, PINS["mods"])):
        raise ValueError("Exact mod provenance mismatch")
    if manifest["refusals"] != {"unloadedChunk": "unloaded_chunk", "snapshotMismatch": "snapshot_mismatch", "contextMismatch": "context_mismatch"}:
        raise ValueError("Required runtime refusal evidence missing")
    artifacts = manifest["inputs"]["gameArtifacts"]
    if any(not any(all(a.get(k) == pin[k] for k in ["file","bytes","sha256"]) for a in artifacts) for pin in PINS["gameArtifacts"]):
        raise ValueError("Pinned game artifacts missing from capture")
    materials, meshes, objects = manifest["materials"], manifest["meshes"], manifest["objects"]
    if not 0 < len(materials) <= 64 or not 0 < len(meshes) <= 128 or not 0 < len(objects) <= 144:
        raise ValueError("Capture count limit")
    roles = {o["role"]: (i, o) for i, o in enumerate(objects) if o["role"] != "surroundings"}
    if set(roles) != {"diamond_pedestal", "emerald_pedestal", "custom_loader_barnacles"}:
        raise ValueError("Required fixture roles missing")
    left, right = roles["diamond_pedestal"][1], roles["emerald_pedestal"][1]
    if left["state"] != right["state"] or min(left["blockEntityVertices"], right["blockEntityVertices"]) < 4:
        raise ValueError("Identical-state contextual block geometry missing")
    if [left["position"], right["position"], roles["custom_loader_barnacles"][1]["position"]] != [[3,65,3], [6,65,3], [9,65,3]]:
        raise ValueError("Fixture coordinates mismatch")
    if left["displayedItem"] != "minecraft:diamond" or right["displayedItem"] != "minecraft:emerald":
        raise ValueError("Wrong contextual display items")
    if roles["custom_loader_barnacles"][1]["loader"] != "supplementaries:random_rotation":
        raise ValueError("Wrong custom loader")
    if "supplementaries" not in roles["custom_loader_barnacles"][1]["bakedModelClass"]:
        raise ValueError("Custom model was not baked by the real mod")
    files = manifest["files"]
    if len(files) > 145 or set(p.name for p in directory.iterdir()) != set(files) | {"capture.json"}:
        raise ValueError("Unlisted files or export count exceeded")
    total = len(raw)
    verified = {}
    image_bytes = 0
    for name, receipt in files.items():
        if not FILES.fullmatch(name):
            raise ValueError("Invalid artifact name")
        data = regular(directory / name, 32 * 1024 * 1024)
        total += len(data)
        if total > MAX_EXPORT or len(data) != receipt["bytes"] or digest(data) != receipt["sha256"]:
            raise ValueError("Artifact size/checksum or aggregate limit failure")
        if name.endswith(".png"):
            w, h = png(data)
            image_bytes += w * h * 4
            if image_bytes > MAX_EXPORT:
                raise ValueError("Aggregate decoded image budget exceeded")
        verified[name] = data
    vertex_total = triangle_total = 0
    role_hashes = {role: [] for role in roles}
    referenced = {"minecraft-frame.png"}
    for material in materials:
        if (material["mode"] not in {"opaque", "cutout", "translucent"} or material["emissive"] is not False
                or type(material["cull"]) is not bool or type(material["depthWrite"]) is not bool
                or type(material["directionalLighting"]) is not bool
                or material["alphaThreshold"] != (0.1 if material["mode"] == "cutout" else 0)) :
            raise ValueError("Unrepresentable material")
        if material["texture"] not in verified or not material["texture"].startswith("texture-"):
            raise ValueError("Material texture missing")
        if material["uvConvention"] != "gpu_texture_rows_no_flip" or material["lighting"] != "baked_client_lightmap_and_vertex_tint":
            raise ValueError("Unsupported material coordinate/lighting convention")
        referenced.add(material["texture"])
    seen = set()
    for mesh in meshes:
        if mesh["file"] in seen:
            raise ValueError("Duplicate mesh reference")
        seen.add(mesh["file"])
        referenced.add(mesh["file"])
        if mesh["file"] not in verified or not 0 <= mesh["material"] < len(materials) or not 0 <= mesh["object"] < len(objects):
            raise ValueError("Mesh reference invalid")
        data = json.loads(verified[mesh["file"]])
        count = mesh["vertices"]
        vertex_total += count
        triangle_total += mesh["triangles"]
        if count <= 0 or vertex_total > 200000:
            raise ValueError("Vertex limit")
        for key, components in [("positions",3), ("uv",2), ("colors",4), ("normals",3)]:
            values = data[key]
            if len(values) != count * components or any(type(v) not in {int,float} or not math.isfinite(v) for v in values):
                raise ValueError("Invalid finite vertex attributes")
        if len(data["indices"]) != mesh["triangles"] * 3 or any(type(i) is not int or not 0 <= i < count for i in data["indices"]):
            raise ValueError("Invalid triangle indices")
        positions = data["positions"]
        if any(not 0 <= positions[i] <= 12 or not 62 <= positions[i+1] <= 70 or not 0 <= positions[i+2] <= 7 for i in range(0, len(positions), 3)):
            raise ValueError("Geometry exceeds fixture bounds")
        if any(not 0 <= c <= 1 for c in data["colors"]):
            raise ValueError("Invalid vertex color")
        for role, (index, _) in roles.items():
            if mesh["object"] == index:
                # Ignore world translation so a pair cannot differ only because of its coordinates.
                pos = objects[index]["position"]
                normalized = {**data, "positions": [round(v - pos[i % 3], 6) for i, v in enumerate(positions)]}
                role_hashes[role].append(digest(json.dumps(normalized, sort_keys=True).encode()))
    if referenced != set(files) or any(not hashes for hashes in role_hashes.values()):
        raise ValueError("Unreferenced artifacts or required geometry missing")
    if role_hashes["diamond_pedestal"] == role_hashes["emerald_pedestal"]:
        raise ValueError("Context pair has identical emitted appearance")
    report = {"format": manifest["format"], "captureSha256": digest(raw),
              "snapshotSha256": manifest["snapshotSha256"], "contextSha256": manifest["contextSha256"],
              "resourceContextSha256": manifest["resourceContextSha256"], "prototypeJarSha256": manifest["buildReceipt"]["prototypeJarSha256"],
              "artifactBytes": total, "vertices": vertex_total, "triangles": triangle_total,
              "savedFrame": {"tick": manifest["gameTick"], "partialTick": 0, "capturedAt": manifest["capturedAt"]},
              "refusals": manifest["refusals"], "roleAppearanceHashes": role_hashes,
              "validation": "passed", "visualAcceptance": "pending",
              "scope": "isolated Minecraft 1.21.1 / NeoForge 21.1.251 / pinned Supplementaries fixture"}
    return manifest, report


def capture(work: Path) -> tuple[Path, dict, dict]:
    name = regular(work / "exports/current", 80).decode().strip()
    if not NAME.fullmatch(name):
        raise ValueError("Invalid publication pointer")
    directory = work / "exports" / name
    manifest, report = validate(directory)
    return directory, manifest, report


def viewer_server(work: Path, port: int) -> ThreadingHTTPServer:
    vendor = ROOT.parents[2] / "clients/desktop-web/node_modules/three"
    if json.loads(regular(vendor / "package.json", 100000))["version"] != "0.180.0":
        raise ValueError("Proof viewer requires repository three 0.180.0; install clients/desktop-web dependencies")
    routes = {"/": ROOT / "viewer/index.html", "/viewer.js": ROOT / "viewer/viewer.js", "/style.css": ROOT / "viewer/style.css",
              "/vendor/three.module.js": vendor / "build/three.module.js", "/vendor/three.core.js": vendor / "build/three.core.js",
              "/vendor/OrbitControls.js": vendor / "examples/jsm/controls/OrbitControls.js"}
    class Handler(BaseHTTPRequestHandler):
        def send(self, status: int, data: bytes, mime: str) -> None:
            self.send_response(status)
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:
            try:
                path = self.path.split("?",1)[0]
                if path in routes:
                    mime = "text/html" if path == "/" else "text/css" if path.endswith(".css") else "text/javascript"
                    return self.send(200, regular(routes[path], 4*1024*1024), mime)
                if path == "/api/current":
                    directory, manifest, report = capture(work)
                    return self.send(200, json.dumps({"id": directory.name, "manifest": manifest, "report": report}).encode(), "application/json")
                match = re.fullmatch(r"/capture/(capture-[0-9a-f-]{36})/([a-z0-9.-]+)", path)
                if match and NAME.fullmatch(match[1]) and FILES.fullmatch(match[2]):
                    directory = work / "exports" / match[1]
                    manifest, _ = validate(directory)
                    if match[2] in manifest["files"]:
                        return self.send(200, regular(directory / match[2], 32*1024*1024), "image/png" if match[2].endswith(".png") else "application/json")
                self.send(404,b"No published capture. Run /mscproof setup then /mscproof capture in the isolated client.","text/plain")
            except (OSError, ValueError, KeyError, TypeError, zlib.error) as error:
                self.send(409,str(error).encode(),"text/plain")

        def do_POST(self) -> None:
            try:
                # The proof is loopback-only and refuses cross-origin evidence writes.
                if self.path != "/api/evidence" or self.headers.get("Origin") != f"http://127.0.0.1:{port}":
                    return self.send(403,b"Origin refused","text/plain")
                size = int(self.headers.get("Content-Length", "0"))
                if not 0 < size <= 12 * 1024 * 1024:
                    raise ValueError("Evidence body size refused")
                data = json.loads(self.rfile.read(size))
                directory, manifest, report = capture(work)
                if data["captureSha256"] != report["captureSha256"] or data["confirmations"] != {"customLoader": True,"contextPair": True,"savedFrame": True}:
                    raise ValueError("Confirmation does not bind all required appearances to current capture")
                image = base64.b64decode(data["image"], validate=True)
                if len(image) > 8*1024*1024:
                    raise ValueError("Screenshot limit")
                png(image,4096)
                evidence = work / "evidence"
                if evidence.is_symlink():
                    raise ValueError("Evidence link refused")
                evidence.mkdir(exist_ok=True)
                name = directory.name + "-viewer.png"
                (evidence / name).write_bytes(image)
                report.update({"visualAcceptance":"owner_confirmed", "confirmedBy":"Cameron", "confirmations":data["confirmations"],
                               "viewerScreenshot":{"file":name,"sha256":digest(image)}})
                atomic_json(evidence / (directory.name + ".json"), report)
                self.send(200,json.dumps(report).encode(),"application/json")
            except (OSError,ValueError,KeyError,TypeError, zlib.error) as error:
                self.send(409,str(error).encode(),"text/plain")
    return ThreadingHTTPServer(("127.0.0.1",port),Handler)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action",choices=["prepare","build","validate","view","run"])
    parser.add_argument("--workspace",type=Path,default=Path.home()/".cache/msc-map-client-proof")
    parser.add_argument("--source-mods",type=Path)
    parser.add_argument("--gradle")
    parser.add_argument("--port",type=int,default=8766)
    args=parser.parse_args()
    work=args.workspace.expanduser().absolute()
    try:
        if work.is_symlink():
            raise ValueError("Symlink workspace refused")
        if args.action in {"prepare","build","run"}:
            prepare(work,args.source_mods)
        if args.action=="prepare":
            return 0
        if args.action in {"build","run"}:
            executable=gradle(args.gradle)
            build(work,executable)
        if args.action=="build":
            return 0
        if args.action=="validate":
            directory,manifest,report=capture(work)
            atomic_json(work/"validation.json",report)
            print(json.dumps(report,indent=2))
            return 0
        server=viewer_server(work,args.port)
        thread=threading.Thread(target=server.serve_forever,daemon=True)
        thread.start()
        url=f"http://127.0.0.1:{args.port}"
        print(f"Proof viewer: {url}",flush=True)
        if args.action=="run":
            print("Launching only the isolated proof client. Create a NEW Creative Superflat world named 'MSC Mesh Capture Proof', enable commands.\nIn it: /tp @s 6 67 11, wait for chunk 0,0; /mscproof setup, wait for the scene; /mscproof capture.\nIn the viewer: Load capture; compare barnacles and diamond/emerald pedestals with Minecraft, then record your visual evidence.\nExisting worlds are never imported. Closing Minecraft ends this command.",flush=True)
            webbrowser.open(url)
            try:
                return subprocess.call([executable,"--no-daemon","-p",str(ROOT),f"-PproofWorkspace={work}","runClient"])
            finally:
                server.shutdown()
        try:
            while thread.is_alive():
                thread.join(1)
        finally:
            server.shutdown()
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        print(str(error),file=sys.stderr)
        return 1
    return 0

if __name__=="__main__":
    raise SystemExit(main())
