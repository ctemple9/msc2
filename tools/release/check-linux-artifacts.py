#!/usr/bin/env python3
"""Record and enforce the glibc ABI of the exact Linux release payloads."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile


GLIBC_PATTERN = re.compile(r"\bGLIBC_(\d+(?:\.\d+)+)\b")
GUI_LIBRARY_MARKERS = (
    "libgdk",
    "libgtk",
    "libqt",
    "libwayland",
    "libwx_gtk",
    "libx11",
    "libxcb",
    "libxcomposite",
    "libxcursor",
    "libxext",
    "libxi",
    "libxinerama",
    "libxkbcommon",
    "libxrandr",
    "libxrender",
)


def fail(message: str) -> None:
    raise ValueError(message)


def version_tuple(value: str) -> tuple[int, ...]:
    return tuple(int(part) for part in value.split("."))


def run(command: list[str], *, cwd: Path | None = None) -> str:
    try:
        result = subprocess.run(
            command, check=True, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
    except FileNotFoundError as error:
        fail(f"required release inspection tool is missing: {command[0]} ({error})")
    except subprocess.CalledProcessError as error:
        fail(
            f"command failed ({' '.join(command)}): "
            f"{error.stderr.strip() or error.stdout.strip()}"
        )
    return result.stdout


def inspect_elf(path: Path) -> tuple[list[str], list[str]]:
    versions_output = run(["readelf", "--wide", "--version-info", str(path)])
    versions = sorted(
        {match.group(1) for match in GLIBC_PATTERN.finditer(versions_output)},
        key=version_tuple,
    )
    dynamic_output = run(["readelf", "--dynamic", "--wide", str(path)])
    needed = re.findall(r"Shared library: \[([^]]+)\]", dynamic_output)
    return versions, needed


def inspect_binary(path: Path, *, label: str, max_glibc: str, headless: bool) -> dict[str, object]:
    if not path.is_file() or path.read_bytes()[:4] != b"\x7fELF":
        fail(f"{label}: expected an ELF executable at {path}")
    versions, needed = inspect_elf(path)
    newest = max(versions, key=version_tuple, default="none")
    if newest != "none" and version_tuple(newest) > version_tuple(max_glibc):
        fail(f"{label}: requires GLIBC_{newest}, above the supported GLIBC_{max_glibc} ceiling")
    if headless:
        gui_libraries = [
            name
            for name in needed
            if any(marker in name.lower() for marker in GUI_LIBRARY_MARKERS)
        ]
        if gui_libraries:
            fail(f"{label}: headless binary links GUI libraries: {', '.join(gui_libraries)}")
    return {
        "path": path.name,
        "required_glibc_versions": versions,
        "highest_glibc_version": newest,
        "needed_libraries": needed,
    }


def archive_binary(archive: Path, destination: Path) -> Path:
    try:
        with tarfile.open(archive, "r:gz") as bundle:
            matches = [
                member
                for member in bundle.getmembers()
                if member.name.removeprefix("./") == "msc"
            ]
            if len(matches) != 1 or not matches[0].isfile():
                fail(f"{archive}: expected exactly one root-level msc executable")
            source = bundle.extractfile(matches[0])
            if source is None:
                fail(f"{archive}: could not read the msc executable")
            output = destination / "msc"
            output.write_bytes(source.read())
            output.chmod(0o755)
            return output
    except (tarfile.TarError, OSError) as error:
        fail(f"could not inspect headless archive {archive}: {error}")


def extract_deb(package: Path, destination: Path) -> None:
    run(["dpkg-deb", "--extract", str(package.resolve()), str(destination)])


def extract_rpm(package: Path, destination: Path) -> None:
    try:
        with tempfile.TemporaryFile() as cpio_stream:
            rpm_result = subprocess.run(
                ["rpm2cpio", str(package.resolve())],
                stdout=cpio_stream,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
            if rpm_result.returncode != 0:
                fail(
                    f"rpm2cpio failed for {package} (exit {rpm_result.returncode}): "
                    f"{rpm_result.stderr.strip() or 'no diagnostic output'}"
                )

            cpio_stream.seek(0)
            extraction = subprocess.run(
                ["cpio", "--extract", "--make-directories", "--quiet"],
                cwd=destination,
                stdin=cpio_stream,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
    except FileNotFoundError as error:
        fail(f"required RPM inspection tool is missing: {error.filename}")

    if extraction.returncode != 0:
        diagnostic = extraction.stderr.strip() or extraction.stdout.strip() or "no diagnostic output"
        fail(f"cpio extraction failed for {package} (exit {extraction.returncode}): {diagnostic}")


def package_binaries(root: Path, label: str, max_glibc: str) -> dict[str, object]:
    binaries = [
        path
        for path in root.rglob("*")
        if path.is_file() and not path.is_symlink() and path.read_bytes()[:4] == b"\x7fELF"
    ]
    if not binaries:
        fail(f"{label}: package contains no ELF executable or shared object")
    inspected = [
        {
            "relative_path": path.relative_to(root).as_posix(),
            **inspect_binary(path, label=f"{label}:{path.relative_to(root)}", max_glibc=max_glibc, headless=False),
        }
        for path in sorted(binaries)
    ]
    versions = sorted(
        {
            version
            for binary in inspected
            for version in binary["required_glibc_versions"]
        },
        key=version_tuple,
    )
    return {"elf_payloads": inspected, "required_glibc_versions": versions}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--headless-archive", type=Path, required=True)
    parser.add_argument("--desktop-deb", type=Path, required=True)
    parser.add_argument("--desktop-rpm", type=Path, required=True)
    parser.add_argument("--max-glibc", default="2.36", help="highest supported GNU libc ABI")
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    for path in (args.headless_archive, args.desktop_deb, args.desktop_rpm):
        if not path.is_file():
            print(f"Linux artifact is missing: {path}", file=sys.stderr)
            return 1

    try:
        with tempfile.TemporaryDirectory(prefix="msc2-linux-abi-") as temporary:
            root = Path(temporary)
            archive_path = archive_binary(args.headless_archive, root)
            headless = inspect_binary(
                archive_path,
                label=args.headless_archive.name,
                max_glibc=args.max_glibc,
                headless=True,
            )

            deb_root = root / "deb"
            rpm_root = root / "rpm"
            deb_root.mkdir()
            rpm_root.mkdir()
            extract_deb(args.desktop_deb, deb_root)
            extract_rpm(args.desktop_rpm, rpm_root)
            report = {
                "supported_floor": "Debian 12 (Bookworm), glibc 2.36",
                "maximum_glibc_abi": args.max_glibc,
                "artifacts": {
                    "headless_archive": {
                        "file": args.headless_archive.name,
                        **headless,
                    },
                    "desktop_deb": {
                        "file": args.desktop_deb.name,
                        **package_binaries(deb_root, args.desktop_deb.name, args.max_glibc),
                    },
                    "desktop_rpm": {
                        "file": args.desktop_rpm.name,
                        **package_binaries(rpm_root, args.desktop_rpm.name, args.max_glibc),
                    },
                },
            }
    except ValueError as error:
        print(f"Linux artifact ABI check failed: {error}", file=sys.stderr)
        return 1

    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"OK: Linux artifacts meet GLIBC_{args.max_glibc} or earlier")
    for kind, artifact in report["artifacts"].items():
        requirements = artifact.get("required_glibc_versions", [])
        highest = max(requirements, key=version_tuple, default="none")
        print(f"{kind}: highest required GLIBC version {highest}")
    print(f"Recorded exact artifact ABI details at {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
