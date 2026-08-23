#!/usr/bin/env python3
"""Fail closed when Phase 10 Bedrock production wiring is detached.

This is a source-level guard for the integration proof. It deliberately does
not start a server, download BDS, or contact a provider: the production smoke
owns executable coverage, while this check catches a regression where the
smoke or a public response stops using the real router.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class ProductionCheckError(Exception):
    """A human-readable production-wiring failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ProductionCheckError(message)


def read(relative: str) -> str:
    path = ROOT / relative
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise ProductionCheckError(f"{relative}: cannot read ({error})") from error


def require_markers(relative: str, markers: tuple[str, ...]) -> str:
    text = read(relative)
    missing = [marker for marker in markers if marker not in text]
    require(not missing, f"{relative}: missing required markers {missing}")
    return text


def check_no_literal_refusals() -> None:
    # Test names and historical notes may mention an old refusal. The guard
    # scans only production Rust, where such a literal would be user-visible.
    production_files = sorted((ROOT / "crates/msc-agent/src").rglob("*.rs"))
    refusal = re.compile(
        r"(?i)(?:\bbedrock\b[^\n]{0,160}\b(?:not implemented|not supported|unsupported)\b|"
        r"\b(?:not implemented|not supported|unsupported)\b[^\n]{0,160}\bbedrock\b)"
    )
    matches = []
    for path in production_files:
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if refusal.search(line):
                matches.append(f"{path.relative_to(ROOT)}:{number}: {line.strip()}")
    require(
        not matches,
        "production source contains a literal Bedrock refusal:\n" + "\n".join(matches),
    )


def check_runtime_selection() -> None:
    main = require_markers(
        "crates/msc-agent/src/main.rs",
        (
            "BedrockRuntimeSelection::production(app_config)",
            "with_app_config_and_auth_and_bedrock(",
            "bedrock_runtime,",
        ),
    )
    require("BedrockRuntimeSelection::production(app_config)" in main, "main: Bedrock production selection is detached")

    selection = require_markers(
        "crates/msc-agent/src/routes/bedrock_runtime.rs",
        (
            "BedrockRuntimeEligibility::detect",
            "BedrockHost::current()",
            "BedrockRuntimeHandle::Linux",
            "BedrockRuntimeHandle::Windows",
            "BedrockRuntimeHandle::Macos",
            "pub fn state_dto(&self) -> BedrockRuntimeStateDto",
            "backend: self.eligibility.backend.map(backend_dto)",
        ),
    )
    require(
        "pub fn production(app_config: &AgentAppConfigStore)" in selection,
        "runtime selection: production constructor is missing",
    )

    capabilities = require_markers(
        "crates/msc-agent/src/routes/capabilities.rs",
        (
            "networking.lifecycle.bedrock_runtime_state()",
            'bedrock_runtime.state == "available"',
            "backend: bedrock_backend",
            "runtime: Some(bedrock_runtime)",
        ),
    )
    require(
        "bedrock_backend = bedrock_runtime.backend" in capabilities,
        "capabilities: backend is not derived from the selected runtime",
    )


def check_frozen_runtime_dtos() -> None:
    capabilities = require_markers(
        "crates/msc-api/src/dto/capabilities.rs",
        (
            "pub struct BedrockRuntimeStateDto",
            "pub state: String",
            "pub backend: Option<BedrockBackendDto>",
            "pub host_os: Option<HostOsDto>",
            "pub reason_code: Option<String>",
            "pub message: Option<String>",
            "pub help_id: Option<String>",
            "pub struct BedrockSupportDto",
            "pub runtime: Option<BedrockRuntimeStateDto>",
        ),
    )
    require(
        "pub struct CapabilitiesDto" in capabilities,
        "capabilities DTO: frozen CapabilitiesDto is missing",
    )

    # These shared response DTO modules are named by the frozen Phase 10
    # contract. A field in only capabilities is not enough: clients need the
    # same state when reading or mutating an active Bedrock server elsewhere.
    dto_files = (
        "crates/msc-api/src/dto/lifecycle.rs",
        "crates/msc-api/src/dto/provisioning.rs",
        "crates/msc-api/src/dto/status.rs",
        "crates/msc-api/src/dto/settings.rs",
        "crates/msc-api/src/dto/backups.rs",
        "crates/msc-api/src/dto/versions.rs",
    )
    for relative in dto_files:
        text = require_markers(
            relative,
            ("use super::BedrockRuntimeStateDto;", "runtime: Option<BedrockRuntimeStateDto>"),
        )
        require("serde(rename_all = \"camelCase\")" in text, f"{relative}: runtime DTO is not camelCase-wired")

    require_markers(
        "crates/msc-agent/src/routes/bedrock.rs",
        (
            "struct PlayersResponse",
            "struct AllowlistResponse",
            "struct AllowlistMutationResult",
            "runtime: Option<BedrockRuntimeStateDto>",
            "state.bedrock_runtime_state()",
        ),
    )


def check_production_smoke() -> None:
    smoke = require_markers(
        "tools/phase10/phase10-smoke.sh",
        (
            "--synthetic",
            "cargo nextest run -p msc-agent --test bedrock_production_smoke",
        ),
    )
    require("--synthetic" in smoke, "smoke: production smoke is not synthetic-only")

    test = require_markers(
        "crates/msc-agent/tests/bedrock_production_smoke.rs",
        (
            "ProductionBackend::current()",
            "ProductionFixture::new()",
            "fixture.spawn_agent()",
            '"/v1/capabilities"',
            '"/v1/start"',
            '"/v1/stop"',
        ),
    )
    require(
        "bedrock_smoke::spawn(" not in test,
        "smoke: production test bypasses the production fixture",
    )

    support = require_markers(
        "crates/msc-agent/tests/support/bedrock_smoke.rs",
        (
            "pub struct ProductionFixture",
            "pub fn spawn_agent(&self)",
            "env!(\"CARGO_BIN_EXE_msc\")",
            '.args(["serve",',
            "MSC2_APP_CONFIG_PATH",
            "127.0.0.1",
        ),
    )

    # These names would move the CI proof outside its committed offline
    # fixture boundary. Local loopback HTTP is intentionally allowed above.
    forbidden = (
        "bedrock-server-downloads",
        "kittizz",
        "minecraft.net",
        "mojang.com",
        "playit.gg",
        "Virtualization.framework",
        "VZVirtualMachine",
        "curl ",
        "wget ",
    )
    combined = smoke + test + support
    found = [marker for marker in forbidden if marker.lower() in combined.lower()]
    require(not found, f"smoke: offline boundary was widened by {found}")


def check() -> None:
    check_no_literal_refusals()
    check_runtime_selection()
    check_frozen_runtime_dtos()
    check_production_smoke()
    print("PHASE 10 PRODUCTION WIRING CHECK PASSED")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check production Bedrock wiring")
    args = parser.parse_args()
    if not args.check:
        parser.error("choose --check")
    try:
        check()
    except (ProductionCheckError, OSError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
