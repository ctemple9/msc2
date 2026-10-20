#!/usr/bin/env python3
"""Build and stage MSC's pinned Vantage renderer for this host."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "third_party" / "vantage"
PIN = json.loads(Path(__file__).with_name("vantage-release.json").read_text())
ZIG_VERSION = PIN["zig_version"]
LICENSE = Path(__file__).with_name("VANTAGE-LICENSE.txt")


def native_platform() -> str:
    system = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}.get(platform.system())
    machine = {"x86_64": "x86_64", "AMD64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}.get(platform.machine())
    if not system or not machine:
        raise SystemExit("unsupported renderer build host")
    return f"{system}-{machine}"


def zig_binary() -> Path:
    override = os.environ.get("MSC2_ZIG_BIN")
    if override:
        candidate = Path(override)
        if not candidate.is_file():
            raise SystemExit(f"MSC2_ZIG_BIN does not exist: {candidate}")
    else:
        cached = ROOT / "target" / "toolchains" / f"ziglang-{ZIG_VERSION}"
        name = "zig.exe" if os.name == "nt" else "zig"
        candidate = cached / "ziglang" / name
        if not candidate.is_file():
            cached.mkdir(parents=True, exist_ok=True)
            subprocess.run(
                [sys.executable, "-m", "pip", "install", "--disable-pip-version-check",
                 "--no-deps", "--only-binary=:all:", "--target", str(cached),
                 f"ziglang=={ZIG_VERSION}"],
                check=True,
            )
    actual = subprocess.check_output([str(candidate), "version"], text=True).strip()
    if actual != ZIG_VERSION:
        raise SystemExit(f"renderer requires Zig {ZIG_VERSION}; found {actual}")
    return candidate


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=PIN["platforms"], required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    if args.platform != native_platform():
        raise SystemExit(f"renderer build requires native {args.platform} host")
    subprocess.run([str(zig_binary()), "build", "-Doptimize=ReleaseFast"], cwd=SOURCE, check=True)
    executable = "vantage.exe" if os.name == "nt" else "vantage"
    built = SOURCE / "zig-out" / "bin" / executable
    if not built.is_file():
        raise SystemExit(f"renderer build produced no executable: {built}")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    temporary = args.output_dir / f".{executable}.staging"
    shutil.copy2(built, temporary)
    if os.name != "nt":
        temporary.chmod(0o755)
    temporary.replace(args.output_dir / executable)
    shutil.copy2(LICENSE, args.output_dir / LICENSE.name)
    print(f"staged MSC Vantage {PIN['upstream_version']} cache format {PIN['cache_format']} for {args.platform}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
