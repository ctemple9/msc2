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
import csv
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
KERNEL_SHA256 = "85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e"
INITRAMFS_SHA256 = "4a67a927c406ff45fa64ad00dc1b541a13d8b7bb0a1d40258697c28731166bb2"
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


def check_source_fragments(required_by_file: dict[str, list[str]]) -> list[str]:
    problems: list[str] = []
    for relative, required in required_by_file.items():
        text = read_text(relative)
        problems.extend(
            f"{relative}: missing {fragment!r}"
            for fragment in missing_fragments(text, required)
        )
    return problems


def check_runtime_selection() -> list[str]:
    return check_source_fragments(
        {
            "crates/msc-application/src/bedrock_runtime.rs": [
                "pub fn detect(fs: &dyn FileSystem, paths: &BedrockRuntimePaths)",
                "BedrockHost::Linux => Self::native(fs, host, paths, BedrockPlatform::Linux)",
                "BedrockHost::Windows => Self::native(fs, host, paths, BedrockPlatform::Windows)",
                "BedrockHost::MacosIntel => Self::sidecar(fs, host, paths)",
                "BedrockHost::MacosAppleSilicon => Self {",
                'reason_code: Some("no_test_hardware".to_owned())',
                'message: "Bedrock is unavailable on Apple Silicon under D-028."',
                "pub trait BedrockRuntime",
                "fn provision(&mut self, request: BedrockProvisionRequest)",
                "fn start(&mut self, request: BedrockStartRequest)",
                "fn stop(&mut self) -> Result<(), BedrockRuntimeError>",
                "fn command(&mut self, command: &str)",
                "fn poll_event(&mut self)",
            ],
            "crates/msc-agent/src/routes/bedrock_runtime.rs": [
                "pub fn production(app_config: &AgentAppConfigStore)",
                "BedrockHost::Linux => {",
                "BedrockRuntimeHandle::Linux(Box::new(runtime))",
                "BedrockHost::Windows => {",
                "BedrockRuntimeHandle::Windows(Box::new(runtime))",
                "BedrockHost::MacosIntel => {",
                "BedrockRuntimeHandle::Macos(Box::new(runtime))",
                "BedrockRuntimeHandle::Unavailable",
                "pub fn state_dto(&self) -> BedrockRuntimeStateDto",
                "pub fn refresh_for_server(&self, server_dir: impl AsRef<Path>)",
                "pub fn ensure_distribution(",
                "(BedrockHost::Linux, Some(BedrockRuntimeBackend::Native)) => Some(BedrockPlatform::Linux)",
                "(BedrockHost::Windows, Some(BedrockRuntimeBackend::Native)) =>",
                "(BedrockHost::MacosIntel, Some(BedrockRuntimeBackend::Sidecar)) =>",
                'std::env::var_os("MSC2_BEDROCK_SIDECAR_DIR")',
                'root.join("BedrockSidecar")',
                'root.join("vmlinuz-kata")',
                'root.join("appliance-initramfs.gz")',
            ],
            "crates/msc-application/src/bedrock_linux.rs": [
                "pub struct LinuxBedrockRuntime",
                "impl<C: BedrockRuntimeClock> BedrockRuntime for LinuxBedrockRuntime",
                "bedrock_native::linux_bedrock_spawn_request(server_dir)",
                "bedrock_native::preflight_udp_bind(",
                "BedrockRuntimeState::Starting",
                "BedrockRuntimeEvent::Ready",
                "BedrockRuntimeEvent::Terminated",
            ],
            "crates/msc-application/src/bedrock_windows.rs": [
                "pub struct WindowsBedrockRuntime",
                "impl<C: BedrockRuntimeClock> BedrockRuntime for WindowsBedrockRuntime",
                "bedrock_native::windows_bedrock_spawn_request(server_dir)",
                "bedrock_native::preflight_udp_bind(",
                "BedrockRuntimeState::Starting",
                "BedrockRuntimeEvent::Ready",
                "BedrockRuntimeEvent::Terminated",
            ],
            "crates/msc-application/src/bedrock_macos.rs": [
                "pub struct MacosBedrockRuntime",
                "impl<T: SidecarTransport> BedrockRuntime for MacosBedrockRuntime<T>",
                "self.inner.provision(request)",
                "self.inner.start(request)",
                "self.inner.stop()",
                "self.inner.command(command)",
                "self.inner.poll_event()",
            ],
            "crates/msc-infrastructure/src/bedrock_native.rs": [
                "pub const BEDROCK_EXECUTABLE_NAME: &str = \"bedrock_server\"",
                "pub const WINDOWS_BEDROCK_EXECUTABLE_NAME: &str = \"bedrock_server.exe\"",
                "pub const BEDROCK_BIND_ADDRESS",
                "pub fn preflight_udp_bind(",
                "pub fn linux_bedrock_spawn_request(",
                "pub fn windows_bedrock_spawn_request(",
            ],
        }
    )


def check_verified_provisioning() -> list[str]:
    return check_source_fragments(
        {
            "crates/msc-infrastructure/src/bedrock_distribution.rs": [
                "pub enum BedrockPlatform",
                "pub fn inspect_installed_distribution(",
                "pub fn resolve_release(",
                "sha256",
                "pub fn stage_archive(",
                "BEDROCK_PROVENANCE_MARKER",
                "ArchiveMissingExecutable",
                "enclosed_name()",
            ],
            "crates/msc-application/src/bedrock_provisioning.rs": [
                "pub fn ensure_installed(",
                "request.platform",
                "bedrock_distribution::resolve_release(",
                "bedrock_distribution::stage_archive(",
                "const PRESERVED_FILES: [&str; 4]",
                '"server.properties"',
                '"allowlist.json"',
                '"permissions.json"',
                '"whitelist.json"',
                'child_relative == Path::new("worlds")',
                "write_provenance(",
                "BEDROCK_PROVENANCE_MARKER",
                "atomic_write(",
                "fs.rename(&candidate, server_dir)",
            ],
            "crates/msc-agent/src/routes/bedrock_runtime.rs": [
                "self.refresh_for_server(&server_dir);",
                "msc_application::bedrock_provisioning::ensure_installed(",
                "let refreshed = BedrockRuntimeEligibility::for_host(",
                "if refreshed.state != BedrockRuntimeEligibilityState::Available",
                "self.ensure_distribution(request.clone(), pre_downgrade_backup)?;",
            ],
        }
    )


def check_shared_lifecycle() -> list[str]:
    problems = check_source_fragments(
        {
            "crates/msc-agent/src/main.rs": [
                '.route("/servers/create", post(routes::servers::create))',
                '.route("/active-server", post(routes::lifecycle::active_server))',
                '.route("/start", post(routes::lifecycle::start))',
                '.route("/stop", post(routes::lifecycle::stop))',
                '.route("/command", post(routes::commands::command))',
                '.route("/status", get(routes::status::status))',
                '.route("/capabilities", get(routes::capabilities::capabilities))',
            ],
            "crates/msc-agent/src/routes/lifecycle.rs": [
                "if self.active_bedrock_server().is_some()",
                "self.provision_bedrock_server(&active).and_then(|()|",
                "self.inner.bedrock_runtime.start(BedrockStartRequest {",
                "fn spawn_bedrock_pump(&self)",
                "BedrockRuntimeEvent::Ready",
                "BedrockRuntimeEvent::Terminated",
                "self.inner.bedrock_runtime.poll_event()",
                '"capability_unavailable"',
            ],
            "crates/msc-agent/src/routes/commands.rs": [
                "if state.active_bedrock_server().is_some()",
                "state.send_bedrock_command(&command)",
                "runtime: Some(state.bedrock_runtime_state())",
            ],
            "crates/msc-agent/src/routes/status.rs": [
                "pub async fn status(State(state): State<LifecycleRoutesState>)",
                "runtime: state",
                "state.bedrock_runtime_state()",
            ],
            "crates/msc-agent/src/routes/capabilities.rs": [
                "let bedrock_runtime = networking.lifecycle.bedrock_runtime_state();",
                'let bedrock_supported = bedrock_runtime.state == "available";',
                "runtime: Some(bedrock_runtime),",
            ],
            "crates/msc-agent/tests/bedrock_production_lifecycle.rs": [
                "fn production_router_runs_fixture_backed_bedrock_lifecycle()",
                '"/v1/start"',
                '"/v1/command"',
                '"/v1/stop"',
                "fn production_router_provisions_bedrock_before_create_completes()",
                '"/v1/servers/create"',
                '".msc_bds_provenance.json"',
                "fn production_router_reports_unavailable_bedrock_lifecycle()",
            ],
            "crates/msc-agent/tests/bedrock_production_surfaces.rs": [
                "fn production_router_exposes_shared_bedrock_surfaces_and_runtime_errors()",
                '"/v1/settings"',
                '"/v1/versions"',
                '"/v1/players"',
                '"/v1/allowlist"',
                '"/v1/performance"',
                '"/v1/worlds"',
                '"/v1/backups"',
                '"capability_unavailable"',
            ],
        }
    )

    relative = "docs/msc2/api-contract/openapi.json"
    path = ROOT / relative
    if not path.is_file():
        problems.append(f"missing API contract: {relative}")
    else:
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            problems.append(f"could not read {relative}: {error}")
        else:
            paths = document.get("paths", {})
            for route, method in {
                "/v1/servers/create": "post",
                "/v1/start": "post",
                "/v1/stop": "post",
                "/v1/command": "post",
                "/v1/status": "get",
                "/v1/capabilities": "get",
            }.items():
                operation = paths.get(route, {}).get(method)
                if operation is None:
                    problems.append(f"{relative}: missing {method.upper()} {route}")

            schemas = document.get("components", {}).get("schemas", {})
            if "BedrockRuntimeStateDTO" not in schemas:
                problems.append(f"{relative}: missing BedrockRuntimeStateDTO schema")

            def schema_has_runtime(schema_name: str, seen: set[str] | None = None) -> bool:
                seen = set() if seen is None else seen
                if schema_name in seen:
                    return False
                seen.add(schema_name)
                schema = schemas.get(schema_name, {})
                if "runtime" in schema.get("properties", {}):
                    return True
                for value in schema.values():
                    if isinstance(value, dict) and "$ref" in value:
                        reference = value["$ref"]
                        prefix = "#/components/schemas/"
                        if reference.startswith(prefix) and schema_has_runtime(
                            reference[len(prefix) :], seen
                        ):
                            return True
                    elif isinstance(value, (dict, list)):
                        nested = json.dumps(value)
                        if '"$ref": "#/components/schemas/' in nested:
                            for nested_name in schemas:
                                if (
                                    f'"$ref": "#/components/schemas/{nested_name}"'
                                    in nested
                                    and schema_has_runtime(nested_name, seen)
                                ):
                                    return True
                return False

            for route in ("/v1/start", "/v1/stop", "/v1/command", "/v1/status"):
                operation = paths.get(route, {}).get(
                    "post" if route != "/v1/status" else "get", {}
                )
                response = operation.get("responses", {}).get("200", {})
                schema = (
                    response.get("content", {})
                    .get("application/json", {})
                    .get("schema", {})
                )
                reference = schema.get("$ref", "")
                prefix = "#/components/schemas/"
                schema_name = reference[len(prefix) :] if reference.startswith(prefix) else ""
                if not schema_has_runtime(schema_name):
                    problems.append(f"{relative}: {route} does not disclose runtime state")
    return problems


def check_service_and_headless_layout() -> list[str]:
    relative = "packaging/update-release-schema.json"
    path = ROOT / relative
    if not path.is_file():
        return [f"missing release schema: {relative}"]
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"could not read {relative}: {error}"]

    problems: list[str] = []
    expected = {
        "releaseSet": "desktop-agent-sidecar",
        "macos": {"requiredArtifacts": ["desktop", "agent", "sidecar"]},
        "windows": {
            "requiredArtifacts": ["desktop", "agent"],
            "forbiddenArtifacts": ["sidecar"],
        },
        "linux": {"installation": "package-manager-only"},
    }
    for key, value in expected.items():
        if key in ("macos", "windows", "linux"):
            for nested_key, nested_value in value.items():
                if document.get(key, {}).get(nested_key) != nested_value:
                    problems.append(
                        f"{relative}: {key}.{nested_key} must be {nested_value!r}"
                    )
        elif document.get(key) != value:
            problems.append(f"{relative}: {key} must be {value!r}")

    preserved_data = document.get("preservedData", [])
    for item in ("configuration", "secrets", "worlds", "server-files"):
        if item not in preserved_data:
            problems.append(f"{relative}: updates must preserve {item}")

    return problems


def check_matrix_boundary() -> list[str]:
    relative = "docs/msc2/bedrock/compatibility-matrix.csv"
    path = ROOT / relative
    if not path.is_file():
        return [f"missing compatibility matrix: {relative}"]

    expected = {
        "Linux (Debian 12)": ("x86_64", "native-linux-bds"),
        "Windows": ("x86_64", "native-windows-bds"),
        "macOS (Intel)": ("x86_64", "macos-vz-swift-sidecar"),
        "macOS (Apple Silicon)": ("arm64", "macos-vz-swift-sidecar"),
    }
    try:
        with path.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
    except (OSError, csv.Error) as error:
        return [f"could not read {relative}: {error}"]

    problems: list[str] = []
    actual_hosts = {row.get("host", "") for row in rows}
    missing_hosts = sorted(set(expected) - actual_hosts)
    if missing_hosts:
        problems.append(f"{relative}: missing matrix rows {missing_hosts!r}")
    for row in rows:
        host = row.get("host", "")
        if host not in expected:
            problems.append(f"{relative}: unexpected matrix row {host!r}")
            continue
        architecture, backend = expected[host]
        if row.get("architecture") != architecture:
            problems.append(f"{relative}: {host} architecture must be {architecture!r}")
        if row.get("bedrock_backend") != backend:
            problems.append(f"{relative}: {host} backend must be {backend!r}")
        if row.get("bedrock_runtime_status") != "unavailable":
            problems.append(
                f"{relative}: {host} must remain unavailable until live evidence exists"
            )
        for field in ("agent_host_evidence", "bedrock_runtime_evidence"):
            evidence = row.get(field, "")
            if not evidence or not (ROOT / evidence).is_file():
                problems.append(f"{relative}: {host} evidence path is missing: {evidence!r}")
    return problems


def check_readiness_doc() -> list[str]:
    relative = "docs/msc2/bedrock/phase12-readiness.md"
    required = [
        "# Bedrock implementation readiness",
        "implementation-ready",
        "does not claim live Bedrock support",
        "native Linux",
        "native Windows",
        "macOS (Intel)",
        "native-linux-bds",
        "native-windows-bds",
        "macos-vz-swift-sidecar",
        "Apple Silicon",
        "no_test_hardware",
        "verified BDS",
        ".msc_bds_provenance.json",
        "server.properties",
        "allowlist.json",
        "permissions.json",
        "worlds/",
        "launchd LaunchDaemon",
        "Windows Service",
        "systemd",
        "headless",
        "POST /v1/start",
        "POST /v1/stop",
        "POST /v1/command",
        "GET /v1/status",
        "GET /v1/capabilities",
        "compatibility matrix",
        "P12.33 handoff",
        "disposable Bedrock server",
        "UDP reachability",
        "lifecycle recovery",
        "promote only the matching matrix cells",
        "No real server, VM boot, Windows run, or macOS run is required here.",
    ]
    return [
        f"{relative}: missing {fragment!r}"
        for fragment in missing_fragments(read_text(relative), required)
    ]


def readiness() -> tuple[int, list[str]]:
    checks = [
        ("package-contract", check_repo()),
        ("runtime-selection", check_runtime_selection()),
        ("verified-provisioning", check_verified_provisioning()),
        ("shared-lifecycle", check_shared_lifecycle()),
        ("service-headless-layout", check_service_and_headless_layout()),
        ("matrix-boundary", check_matrix_boundary()),
        ("readiness-record", check_readiness_doc()),
    ]
    problems = [
        f"{name}: {problem}"
        for name, check_problems in checks
        for problem in check_problems
    ]
    lines = [
        f"{name}={'pass' if not check_problems else 'fail'}"
        for name, check_problems in checks
    ]
    if problems:
        lines.extend(f"error: {problem}" for problem in problems)
        return 1, lines

    lines.extend(
        [
            "implementation-readiness=pass",
            "live-runtime-evidence=deferred",
            "compatibility-matrix-promotion=deferred",
            "handoff: run one disposable Bedrock server per available host, verify UDP reachability and lifecycle recovery, then promote only matching cells",
        ]
    )
    return 0, lines


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
    if sys.argv[1:] == ["--selftest"]:
        code, lines = selftest()
    elif sys.argv[1:] == ["--readiness"]:
        code, lines = readiness()
    else:
        print(
            "usage: python3 tools/phase12/bedrock-package-check.py "
            "--selftest|--readiness"
        )
        return 2
    print("\n".join(lines))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
