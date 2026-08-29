#!/usr/bin/env python3
"""Focused contract check for the P12.31 Bedrock package layouts.

This is a repository check, not a cross-platform build.  It verifies that the
desktop resource map, service layout, macOS sidecar project, and staging script
describe the same files.  The self-test also exercises the missing-resource and
checksum-failure paths with temporary fixture files, so a broken checker is
visible without requiring macOS, Xcode, or the appliance binaries.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
KERNEL_SHA256 = "85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e"
INITRAMFS_SHA256 = "0865eb432f61249a5a2f76770e7c79e53cf803c5fa435d110ced03747da8a278"
APPLIANCE_CHECKSUMS = {
    "vmlinuz-kata": KERNEL_SHA256,
    "appliance-initramfs.gz": INITRAMFS_SHA256,
}


def read_text(relative: str) -> str | None:
    path = ROOT / relative
    if not path.is_file():
        return None
    return path.read_text(encoding="utf-8")


def missing_fragments(text: str | None, fragments: list[str]) -> list[str]:
    if text is None:
        return fragments
    normalized = " ".join(text.split())
    return [
        fragment
        for fragment in fragments
        if fragment not in text and " ".join(fragment.split()) not in normalized
    ]


def check_layout() -> list[str]:
    relative = "packaging/agent-service-layout.json"
    path = ROOT / relative
    if not path.is_file():
        return [f"missing layout: {relative}"]
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"could not read {relative}: {error}"]

    problems: list[str] = []
    package = document.get("package", {})
    expected_package = {
        "agentResourceSource": "clients/desktop-web/src-tauri/target/package/agent",
        "agentResourceTarget": "agent",
        "bedrockRuntimeArchive": "verified-first-run-download",
        "bedrockRuntimeBundled": False,
    }
    if package != expected_package:
        problems.append(f"{relative}: package resource contract drifted")

    platforms = document.get("platforms", {})
    expected_common = {
        "macos": {
            "manager": "launchd LaunchDaemon",
            "agentPath": "MSC 2.app/Contents/Resources/agent/msc",
            "sidecarPath": "MSC 2.app/Contents/Resources/agent/sidecar",
            "headlessAgentPath": "agent/msc",
            "headlessSidecarPath": "agent/sidecar",
            "dataPath": "~/Library/Application Support/MSC 2",
        },
        "windows": {
            "manager": "Windows Service",
            "agentPath": "agent/msc.exe",
            "sidecarPath": None,
            "dataPath": "%USERPROFILE%\\AppData\\Roaming\\MSC2",
        },
        "linux": {
            "manager": "systemd",
            "agentPath": "../lib/msc2-desktop-web/agent/msc",
            "sidecarPath": None,
            "dataPath": "~/.local/share/msc2",
        },
    }
    for platform, expected in expected_common.items():
        actual = platforms.get(platform)
        if actual is None:
            problems.append(f"{relative}: missing {platform} platform")
            continue
        for key, value in expected.items():
            if actual.get(key) != value:
                problems.append(f"{relative}: {platform}.{key} must be {value!r}")

    mac_bedrock = platforms.get("macos", {}).get("bedrock", {})
    expected_mac_bedrock = {
        "backend": "intel-vm-sidecar",
        "sidecarExecutable": "MSC 2.app/Contents/Resources/agent/sidecar/BedrockSidecar",
        "applianceArchitecture": "x86_64",
        "applianceResources": [
            "MSC 2.app/Contents/Resources/agent/sidecar/vmlinuz-kata",
            "MSC 2.app/Contents/Resources/agent/sidecar/appliance-initramfs.gz",
        ],
        "applianceBuildInput": "MSC2_BEDROCK_APPLIANCE_DIR",
        "runtimeArchive": "verified-first-run-download",
        "runtimeBundled": False,
    }
    for key, value in expected_mac_bedrock.items():
        if mac_bedrock.get(key) != value:
            problems.append(f"{relative}: macos.bedrock.{key} must be {value!r}")

    for platform, executable in (
        ("windows", "server-directory/bedrock_server.exe"),
        ("linux", "server-directory/bedrock_server"),
    ):
        bedrock = platforms.get(platform, {}).get("bedrock", {})
        expected = {
            "backend": "native",
            "executable": executable,
            "runtimeArchive": "verified-first-run-download",
            "runtimeBundled": False,
        }
        for key, value in expected.items():
            if bedrock.get(key) != value:
                problems.append(f"{relative}: {platform}.bedrock.{key} must be {value!r}")
    return problems


def check_tauri_config() -> list[str]:
    relative = "clients/desktop-web/src-tauri/tauri.conf.json"
    path = ROOT / relative
    if not path.is_file():
        return [f"missing Tauri config: {relative}"]
    try:
        config = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"could not read {relative}: {error}"]

    problems: list[str] = []
    build = config.get("build", {})
    if "prepare:agent" not in build.get("beforeDevCommand", ""):
        problems.append(f"{relative}: development builds do not stage the agent")
    if "prepare:agent" not in build.get("beforeBuildCommand", ""):
        problems.append(f"{relative}: release builds do not stage the agent")
    resources = config.get("bundle", {}).get("resources")
    if resources != {"target/package/agent": "agent"}:
        problems.append(f"{relative}: bundle must map target/package/agent to agent")
    return problems


def check_prepare_script() -> list[str]:
    relative = "clients/desktop-web/tools/prepare-agent-dev.mjs"
    text = read_text(relative)
    required = [
        "cargo",
        "msc-agent",
        "packageAgentDirectory",
        "stageFile(source, join(packageAgentDirectory, agentName))",
        "process.platform === 'darwin'",
        "MSC2_BEDROCK_APPLIANCE_DIR",
        "stageMacosSidecar",
        "xcodebuild",
        "BedrockSidecar.xcodeproj",
        "ARCHS=x86_64",
        "ONLY_ACTIVE_ARCH=NO",
        "vmlinuz-kata",
        "appliance-initramfs.gz",
        KERNEL_SHA256,
        INITRAMFS_SHA256,
        "const devSidecarDirectory = join(destinationRoot, 'Resources', 'agent', 'sidecar')",
        "const packageSidecarDirectory = join(packageAgentDirectory, 'sidecar')",
        "stageFile(builtSidecar, join(devSidecarDirectory, 'BedrockSidecar'))",
        "stageFile(join(applianceDirectory, name), join(packageSidecarDirectory, name))",
    ]
    return [f"{relative}: missing {fragment!r}" for fragment in missing_fragments(text, required)]


def check_rust_lookup() -> list[str]:
    relative = "clients/desktop-web/src-tauri/src/lib.rs"
    required = [
        'const BEDROCK_SIDECAR_DIRECTORY_ENV: &str = "MSC2_BEDROCK_SIDECAR_DIR"',
        ".env(BEDROCK_SIDECAR_DIRECTORY_ENV, sidecar_directory.display().to_string())",
        "fn packaged_bedrock_sidecar_directory()",
        "directory.join(\"../Resources/agent/sidecar\")",
        '"BedrockSidecar", "vmlinuz-kata", "appliance-initramfs.gz"',
        'let development_path = directory.join("agent/msc")',
        "../lib/msc2-desktop-web/agent/msc",
    ]
    text = read_text(relative)
    return [f"{relative}: missing {fragment!r}" for fragment in missing_fragments(text, required)]


def check_sidecar_project() -> list[str]:
    relative = "sidecar/bedrock/BedrockSidecar.xcodeproj/project.pbxproj"
    required = [
        "A00000000000000000000007 /* vmlinuz-kata */",
        "A00000000000000000000008 /* appliance-initramfs.gz */",
        'path = "$(MSC2_BEDROCK_APPLIANCE_DIR)/vmlinuz-kata"',
        'path = "$(MSC2_BEDROCK_APPLIANCE_DIR)/appliance-initramfs.gz"',
        "PBXShellScriptBuildPhase",
        "Validate Intel appliance resources",
        r'input=\"${MSC2_BEDROCK_APPLIANCE_DIR:-}\"',
        r'if [ -z \"$input\" ]',
        r'if [ ! -f \"$file\" ]',
        "shasum -a 256",
        "checksum mismatch",
        KERNEL_SHA256,
        INITRAMFS_SHA256,
        "ARCHS = x86_64",
        "ONLY_ACTIVE_ARCH = NO",
        'MSC2_BEDROCK_APPLIANCE_DIR = "$(SRCROOT)/Resources"',
        "files = (A00000000000000000000013, A00000000000000000000016, A00000000000000000000017,)",
        "buildPhases = (A00000000000000000000020, A00000000000000000000021, A00000000000000000000026, A00000000000000000000023,)",
    ]
    text = read_text(relative)
    problems = [f"{relative}: missing {fragment!r}" for fragment in missing_fragments(text, required)]
    if text is not None and "A00000000000000000000032 = {isa = PBXVariantGroup" in text:
        problems.append(f"{relative}: Resources must be a normal group for binary appliance files")
    return problems


def check_resource_readme() -> list[str]:
    relative = "sidecar/bedrock/Resources/README.md"
    required = [
        "Intel (`x86_64`) appliance",
        "MSC2_BEDROCK_APPLIANCE_DIR",
        "vmlinuz-kata",
        "appliance-initramfs.gz",
        KERNEL_SHA256,
        INITRAMFS_SHA256,
        "verified first-run download",
        "do not receive this VM appliance",
        "Apple Silicon",
        "arm64",
    ]
    text = read_text(relative)
    return [f"{relative}: missing {fragment!r}" for fragment in missing_fragments(text, required)]


def validate_appliance_directory(
    directory: Path | None, checksums: dict[str, str] = APPLIANCE_CHECKSUMS
) -> list[str]:
    if directory is None:
        return ["MSC2_BEDROCK_APPLIANCE_DIR is required"]

    problems: list[str] = []
    for name, expected in checksums.items():
        path = directory / name
        if not path.is_file():
            problems.append(f"missing Bedrock appliance resource: {path}")
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            problems.append(
                f"Bedrock appliance checksum mismatch for {name}: expected {expected}, got {actual}"
            )
    return problems


def check_repo() -> list[str]:
    return (
        check_layout()
        + check_tauri_config()
        + check_prepare_script()
        + check_rust_lookup()
        + check_sidecar_project()
        + check_resource_readme()
    )


def selftest() -> tuple[int, list[str]]:
    problems = check_repo()
    with TemporaryDirectory() as temporary:
        directory = Path(temporary)
        missing = validate_appliance_directory(directory)

        kernel = b"synthetic Intel kernel"
        initramfs = b"synthetic appliance initramfs"
        (directory / "vmlinuz-kata").write_bytes(kernel)
        (directory / "appliance-initramfs.gz").write_bytes(initramfs)
        fixture_checksums = {
            "vmlinuz-kata": hashlib.sha256(kernel).hexdigest(),
            "appliance-initramfs.gz": hashlib.sha256(initramfs).hexdigest(),
        }
        valid = validate_appliance_directory(directory, fixture_checksums)
        wrong = validate_appliance_directory(
            directory,
            {
                "vmlinuz-kata": "0" * 64,
                "appliance-initramfs.gz": fixture_checksums["appliance-initramfs.gz"],
            },
        )

    lines = [
        f"fixture-missing={'pass' if missing else 'fail'}",
        f"fixture-checksum={'pass' if wrong else 'fail'}",
        f"fixture-valid={'pass' if not valid else 'fail'}",
    ]
    if not missing:
        problems.append("missing-resource fixture was accepted")
    if not wrong:
        problems.append("wrong-checksum fixture was accepted")
    if valid:
        problems.extend(f"valid fixture: {problem}" for problem in valid)
    if problems:
        lines.extend(f"error: {problem}" for problem in problems)
        return 1, lines

    lines.append("repository: Bedrock package paths, Intel sidecar inputs, and first-run runtime contract are aligned")
    return 0, lines


def main() -> int:
    if sys.argv[1:] != ["--selftest"]:
        print("usage: python3 tools/phase12/bedrock-package-check.py --selftest")
        return 2
    code, lines = selftest()
    print("\n".join(lines))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
