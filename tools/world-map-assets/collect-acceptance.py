#!/usr/bin/env python3
"""Read private report/receipt files and print redacted P18.21 evidence.

No network, subprocesses, installs, fixture changes, tests or game launches.
Missing observations stay pending. Output is evidence for owner review, never
an independent phase review or automatic feature/release approval.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import stat
import sys


FIXTURES = (
    "vanilla-control", "paper-tectonic-control", "purpur-control", "fabric-blocks",
    "forge-blocks", "neoforge-blocks", "mixed-pack", "namespace-collision",
    "dependency-order", "empty-versus-broken", "custom-loader",
    "code-rendered-context", "saved-custom-dimension", "bedrock-control",
)
REPAIRS = (
    "missing-bytes", "corrupt-bytes", "wrong-release", "pack-order", "stale-tiles",
    "stale-context", "custom-loader-repair", "code-rendered-repair",
    "cancel-restart-retry", "offline-online-recovery",
)
CONTROLS = (
    "navigation-depth-dimensions", "players-fly-follow", "refresh-clean-exit",
    "host-slot-isolation", "scene-retained-on-failure", "bounded-retry-reuse",
    "source-unchanged", "compatible-rollback", "package-install-update-rollback",
    "native-windows-service", "native-windows-headless", "headless-without-game-or-gpu",
)
SCENARIOS = (
    "first-use-online", "cache-reuse-offline", "loose-mod-source", "provider-manifest-source",
    "manual-client-counterpart", "resource-version-update", "minimum-game-family",
    "current-game-family", "saved-nether-end", "agent-and-game-responsiveness",
)
CASES = FIXTURES + REPAIRS + CONTROLS + SCENARIOS + ("transport-import",)
DESKTOPS = ("macos-x86_64", "macos-aarch64", "windows-x86_64",
            "linux-fedora-x86_64", "linux-ubuntu-x86_64")
AGENTS = ("macos-x86_64", "macos-aarch64", "windows-x86_64",
          "linux-ubuntu-x86_64", "linux-fedora-x86_64")
PAIRS = tuple((d, a) for d in DESKTOPS[:4] for a in AGENTS[:4]) + (
    ("linux-fedora-x86_64", "linux-fedora-x86_64"),
    ("linux-ubuntu-x86_64", "linux-ubuntu-x86_64"),
)
FLAVORS = ("vanilla", "paper", "purpur", "fabric", "forge", "neoforge")
METRICS = ("firstVisibleMs", "preparationMs", "peakAgentRssBytes",
           "peakRendererRssBytes", "peakHelperRssBytes", "cacheBytes",
           "downloadedBytes", "reusedBytes", "scannedBlocks", "scannedChunks")
CLASSIFICATIONS = {"missing_model", "missing_texture", "unsupported_loader",
                   "unsupported_material", "missing_context", "missing_saved_chunk",
                   "intentional_empty", "model_resolved", "captured_appearance", "selection_unknown",
                   "unsupported_renderer_namespace", "invalid_model"}
HASH = re.compile(r"[0-9a-f]{64}\Z")
ID = re.compile(r"[a-zA-Z0-9_.:-]{1,128}\Z")
RESOURCE = re.compile(r"[a-z0-9_.-]+:[a-z0-9_./-]+\Z")
MAX_JSON = 8 * 1024 * 1024
MAX_ARTIFACT = 8 * 1024 * 1024 * 1024


class EvidenceError(ValueError):
    pass


def require(condition: bool, code: str) -> None:
    if not condition:
        raise EvidenceError(code)


def unique_object(pairs: list) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate_json_key")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise EvidenceError("nonfinite_json_number")


def checked_path(root: Path, name: str) -> Path:
    require(isinstance(name, str) and bool(name), "missing_file_reference")
    # Receipts use portable relative names, including on Windows.
    require("\\" not in name and ":" not in name and not name.startswith("/"),
            "unsafe_file_reference")
    parts = name.split("/")
    require(all(part and part not in {".", ".."} for part in parts), "unsafe_file_reference")
    path = root
    for part in parts:
        path /= part
        require(not path.is_symlink(), "linked_evidence_refused")
    require(stat.S_ISREG(path.stat().st_mode), "regular_evidence_required")
    return path


def json_file(path: Path) -> tuple[dict, str]:
    require(not path.is_symlink() and stat.S_ISREG(path.stat().st_mode),
            "regular_json_required")
    with path.open("rb") as stream:
        raw = stream.read(MAX_JSON + 1)
    require(len(raw) <= MAX_JSON, "json_byte_limit")
    value = json.loads(raw, object_pairs_hook=unique_object, parse_constant=reject_constant)
    require(isinstance(value, dict), "json_object_required")
    return value, hashlib.sha256(raw).hexdigest()


def referenced_json(root: Path, ref: dict) -> tuple[dict, str]:
    # Exported API reports need no manually transcribed digest. If a receipt
    # supplies one, it must match; visual confirmations always bind exact digests.
    if isinstance(ref, str):
        ref = {"file": ref}
    require(isinstance(ref, dict), "json_file_reference_required")
    expected = ref.get("sha256")
    require(expected is None or isinstance(expected, str) and HASH.fullmatch(expected) is not None,
            "invalid_json_checksum")
    value, digest = json_file(checked_path(root, ref.get("file")))
    require(expected is None or digest == expected, "json_checksum_mismatch")
    return value, digest


def artifact(root: Path, ref: dict, maximum: int = MAX_ARTIFACT) -> dict:
    require(isinstance(ref, dict) and HASH.fullmatch(ref.get("sha256", "")) is not None,
            "artifact_checksum_required")
    path = checked_path(root, ref.get("file"))
    before = path.stat()
    require(0 < before.st_size <= maximum, "artifact_byte_limit")
    digest = hashlib.sha256()
    total = 0
    with path.open("rb") as stream:
        while raw := stream.read(1024 * 1024):
            total += len(raw)
            require(total <= maximum, "artifact_byte_limit")
            digest.update(raw)
    after = path.stat()
    require((before.st_size, before.st_mtime_ns, before.st_ino) ==
            (after.st_size, after.st_mtime_ns, after.st_ino) and total == before.st_size,
            "artifact_changed")
    require(digest.hexdigest() == ref["sha256"], "artifact_checksum_mismatch")
    return {"sha256": digest.hexdigest(), "bytes": total}


def integer(value: object) -> bool:
    return type(value) is int and 0 <= value <= 2**63 - 1


def counts(report: dict) -> dict:
    value = report.get("counts")
    require(isinstance(value, dict) and all(k in CLASSIFICATIONS and integer(v)
            for k, v in value.items()), "invalid_classified_counts")
    return value


def failures(report: dict) -> int:
    return sum(v for k, v in counts(report).items()
               if k not in {"model_resolved", "intentional_empty", "captured_appearance"})


def scope(report: dict) -> dict:
    binding = report.get("binding", {})
    require(isinstance(binding, dict) and all(isinstance(binding.get(k), str) and
            ID.fullmatch(binding[k]) for k in
            ("agentHostId", "serverId", "slotId", "worldIncarnation", "revision")),
            "invalid_report_binding")
    dimension, area = report.get("dimension"), report.get("area", {})
    require(isinstance(dimension, str) and RESOURCE.fullmatch(dimension) is not None,
            "invalid_report_dimension")
    require(isinstance(area, dict) and all(isinstance(area.get(k), list) and
            len(area[k]) == 3 and all(type(n) is int for n in area[k]) for k in ("min", "max")),
            "invalid_report_area")
    volume = 1
    for lo, hi in zip(area["min"], area["max"]):
        require(-30_000_000 <= lo <= hi < 30_000_000, "invalid_report_area")
        volume *= hi - lo + 1
    require(volume <= 262_144 and -2048 <= area["min"][1] <= area["max"][1] <= 2047,
            "invalid_report_area")
    for key in ("snapshotId", "resourceGenerationId"):
        require(HASH.fullmatch(report.get(key, "")) is not None, "invalid_report_generation")
    require(report.get("schemaVersion") == 1 and ID.fullmatch(report.get("operationId", "")),
            "invalid_report_identity")
    require(report.get("outcome") in {"checked", "ready", "repaired", "needs_input", "unsupported",
            "partially_repaired", "cancelled", "failed"}, "invalid_report_outcome")
    geometry = report.get("geometryGenerationId")
    require(geometry is None or isinstance(geometry, str) and HASH.fullmatch(geometry),
            "invalid_geometry_identity")
    # Revision may change when resource selection is repaired; world identity cannot.
    return {"binding": {k: binding[k] for k in
            ("agentHostId", "serverId", "slotId", "worldIncarnation")},
            "snapshotId": report["snapshotId"], "dimension": dimension, "area": area}


def technical(report: dict) -> bool:
    geometry = report.get("geometryGenerationId")
    return (report.get("outcome") in {"ready", "repaired"} and
            isinstance(geometry, str) and HASH.fullmatch(geometry) is not None and
            failures(report) == 0 and report.get("omittedIssues") == 0)


def repair(before: dict, after: dict) -> bool:
    evidence = after.get("repair")
    return (scope(before) == scope(after) and failures(before) > 0 and technical(after) and
            after.get("outcome") == "repaired" and isinstance(evidence, dict) and
            evidence.get("sameSavedArea") is True and
            evidence.get("beforeOperationId") == before["operationId"] and
            evidence.get("sourceGenerationId") == before["resourceGenerationId"] and
            evidence.get("targetGenerationId") == after["resourceGenerationId"] and
            evidence.get("beforeCounts") == counts(before) and
            evidence.get("afterCounts") == counts(after))


def native_observation(root: Path, item: dict, session: dict, identities: dict) -> dict:
    """Bedrock and navigation have no Java map-assets report; keep that distinction."""
    require(item.get("case") in CONTROLS + ("bedrock-control",), "native_case_not_a_java_repair")
    native, native_hash = referenced_json(root, item["native"])
    require(native.get("schemaVersion") == 1 and native.get("case") == item["case"] and
            native.get("agentHostId") == session["agent"]["hostId"] and
            native.get("confirmedBy") == "Cameron" and type(native.get("confirmed")) is bool,
            "native_owner_receipt_mismatch")
    manifest, manifest_hash = referenced_json(root, native.get("manifest"))
    require(native.get("manifestSha256") == manifest_hash and bool(manifest), "native_manifest_mismatch")
    # The terrain manifest does not expose Java resource diagnostics or a full
    # snapshot receipt. Context here is the owner's explicit saved-scene receipt.
    snapshot = native.get("snapshotId")
    require(isinstance(snapshot, str) and HASH.fullmatch(snapshot), "native_saved_snapshot_required")
    dimension = native.get("dimension")
    require(isinstance(dimension, str) and RESOURCE.fullmatch(dimension), "invalid_native_dimension")
    named = native.get("namedBlocks")
    require(isinstance(named, list) and 0 < len(named) <= 1000, "native_named_appearances_required")
    for block in named:
        require(isinstance(block, dict) and RESOURCE.fullmatch(block.get("id", "")) and
                isinstance(block.get("position"), list) and len(block["position"]) == 3 and
                all(type(n) is int and abs(n) <= 30_000_000 for n in block["position"]),
                "invalid_native_named_block")
    observed_at = native.get("observedAt")
    require(isinstance(observed_at, str) and observed_at.endswith("Z"), "native_time_missing")
    datetime.fromisoformat(observed_at.replace("Z", "+00:00"))
    images = native.get("captures", {})
    require(isinstance(images, dict) and set(images) == {"map", "minecraft"}, "both_captures_required")
    captures = []
    for role, ref in sorted(images.items()):
        verified = artifact(root, ref, 32 * 1024 * 1024)
        with checked_path(root, ref["file"]).open("rb") as stream:
            signature = stream.read(12)
        require(signature.startswith(b"\x89PNG\r\n\x1a\n") or signature.startswith(b"\xff\xd8\xff") or
                signature[:4] == b"RIFF" and signature[8:] == b"WEBP", "capture_image_required")
        captures.append({"role": role, **verified})
    require(item.get("flavor") in FLAVORS + ("bedrock",), "unknown_native_flavor")
    require(item["case"] != "bedrock-control" or item["flavor"] == "bedrock", "bedrock_flavor_required")
    return {"id": item["id"], "case": item["case"], "flavor": item["flavor"],
            "kind": "native_owner_observation", "desktop": session["desktop"]["platform"],
            "agent": session["agent"]["platform"], "agentRole": session["agent"]["role"],
            "desktopHostSha256": hashlib.sha256(session["desktop"]["hostId"].encode()).hexdigest(),
            "agentHostSha256": hashlib.sha256(session["agent"]["hostId"].encode()).hexdigest(),
            "receiptSha256": native_hash, "manifestSha256": manifest_hash, "snapshotId": snapshot,
            "dimension": dimension, "artifacts": identities, "captures": captures,
            "beforeCounts": None, "afterCounts": None, "metrics": {}, "metricsComplete": False,
            "observedAt": observed_at, "visual": "owner_confirmed" if native["confirmed"] else "owner_rejected",
            "disposition": "evidence_recorded" if native["confirmed"] else "failed"}


def observation(root: Path, item: dict, session: dict, identities: dict) -> dict:
    require(isinstance(item, dict) and ID.fullmatch(item.get("id", "")) is not None,
            "invalid_observation_id")
    case = item.get("case")
    require(case in CASES and item.get("flavor") in FLAVORS + ("bedrock",), "unknown_case_or_flavor")
    if case.startswith("native-windows-"):
        require(session["agent"]["platform"] == "windows-x86_64", "windows_agent_required")
    if case in {"native-windows-headless", "headless-without-game-or-gpu"}:
        require(session["agent"]["role"] in {"headless", "service"}, "headless_agent_required")
    if item.get("native") is not None:
        return native_observation(root, item, session, identities)
    report, report_hash = referenced_json(root, item.get("report"))
    saved_scope = scope(report)
    require(report["binding"]["agentHostId"] == session["agent"]["hostId"], "agent_host_mismatch")
    fixture, fixture_hash = referenced_json(root, item.get("fixture"))
    require(fixture.get("schemaVersion") == 1 and ID.fullmatch(fixture.get("id", "")) is not None,
            "invalid_fixture_identity")
    require(fixture.get("snapshotId") == report["snapshotId"] and
            fixture.get("dimension") == report["dimension"] and fixture.get("area") == report["area"],
            "fixture_saved_scope_mismatch")
    require(fixture.get("gameVersion") == report.get("snapshotMinecraftVersion") and
            isinstance(fixture.get("gameVersion"), str) and ID.fullmatch(fixture["gameVersion"]),
            "fixture_game_version_mismatch")
    for key in ("configSha256", "resourceSelectionSha256", "clientArtifactSha256", "serverArtifactSha256"):
        require(HASH.fullmatch(fixture.get(key, "")) is not None, "fixture_input_identity_missing")
    source_hashes = fixture.get("sourceSha256", [])
    require(isinstance(source_hashes, list) and len(source_hashes) <= 10_000 and
            all(isinstance(h, str) and HASH.fullmatch(h) for h in source_hashes), "invalid_fixture_sources")
    report_sources = report.get("sources", [])
    require(isinstance(report_sources, list) and len(report_sources) <= 10_000,
            "report_source_limit")
    require(set(source_hashes) == {s["sha256"] for s in report_sources}, "fixture_source_mismatch")
    require(all(isinstance(s, dict) and isinstance(s.get("sha256"), str) and HASH.fullmatch(s["sha256"])
            for s in report_sources), "invalid_source_identity")
    named = fixture.get("namedBlocks")
    require(isinstance(named, list) and 0 < len(named) <= 1000, "named_appearances_required")
    for block in named:
        require(isinstance(block, dict) and RESOURCE.fullmatch(block.get("id", "")) and
                isinstance(block.get("state"), dict) and isinstance(block.get("position"), list) and
                len(block["position"]) == 3, "invalid_named_block")
        require(all(type(n) is int and report["area"]["min"][axis] <= n <= report["area"]["max"][axis]
                for axis, n in enumerate(block["position"])), "named_block_outside_scope")
    if item["flavor"] not in {"vanilla", "paper", "purpur", "bedrock"}:
        require(isinstance(fixture.get("loaderVersion"), str) and ID.fullmatch(fixture["loaderVersion"]),
                "fixture_loader_pin_missing")
    if case in SCENARIOS:
        require(fixture.get("scenario") == case, "fixture_scenario_mismatch")
    eligible = technical(report)
    before_counts = None
    before_operation = None
    if item.get("before") is not None:
        before, _ = referenced_json(root, item["before"])
        before_counts, before_operation = counts(before), before.get("operationId")
        eligible = eligible and repair(before, report)
    elif case in REPAIRS:
        eligible = False
    if item.get("rendering") is not None:
        rendering, _ = referenced_json(root, item["rendering"])
        eligible = eligible and (rendering.get("state") == "ready" and rendering.get("stale") is False and
                    rendering.get("generationId") == report.get("geometryGenerationId") and
                    rendering.get("snapshotId") == report["snapshotId"] and
                    rendering.get("resourceGenerationId") == report["resourceGenerationId"])
    else:
        eligible = False
    visual_state, captures, observed_at = "pending", [], None
    if item.get("visual") is not None:
        visual, _ = referenced_json(root, item["visual"])
        require(visual.get("schemaVersion") == 1 and visual.get("confirmedBy") == "Cameron" and
                visual.get("reportSha256") == report_hash and visual.get("fixtureSha256") == fixture_hash and
                visual.get("namedBlocks") == named and type(visual.get("confirmed")) is bool,
                "visual_confirmation_mismatch")
        observed_at = visual.get("observedAt")
        require(isinstance(observed_at, str) and observed_at.endswith("Z"), "visual_time_missing")
        datetime.fromisoformat(observed_at.replace("Z", "+00:00"))
        images = visual.get("captures", {})
        require(isinstance(images, dict) and set(images) == {"map", "minecraft"}, "both_captures_required")
        for role, ref in sorted(images.items()):
            verified = artifact(root, ref, 32 * 1024 * 1024)
            with checked_path(root, ref["file"]).open("rb") as stream:
                signature = stream.read(12)
            require(signature.startswith(b"\x89PNG\r\n\x1a\n") or signature.startswith(b"\xff\xd8\xff") or
                    signature[:4] == b"RIFF" and signature[8:] == b"WEBP", "capture_image_required")
            captures.append({"role": role, **verified})
        visual_state = "owner_confirmed" if visual["confirmed"] else "owner_rejected"
    metrics = {}
    if item.get("metrics") is not None:
        measured, _ = referenced_json(root, item["metrics"])
        require(measured.get("operationId") == report["operationId"] and
                measured.get("reportSha256") == report_hash, "metrics_operation_mismatch")
        for key in METRICS:
            value = measured.get(key)
            if value is not None:
                require(type(value) in {int, float} and math.isfinite(value) and 0 <= value <= 2**63 - 1,
                        "invalid_measured_cost")
                metrics[key] = value
    bundle = artifact(root, item["bundle"]) if item.get("bundle") is not None else None
    if case == "transport-import":
        eligible = eligible and bundle is not None
    checks = item.get("checks", {})
    require(isinstance(checks, dict) and all(k in CONTROLS and type(v) is bool for k, v in checks.items()),
            "invalid_owner_control_checks")
    if case in CONTROLS:
        eligible = eligible and checks.get(case) is True
    disposition = ("failed" if visual_state == "owner_rejected" or not eligible else
                   "evidence_recorded" if visual_state == "owner_confirmed" else "pending_visual")
    # Export an allowlist only: no source paths, diagnostics prose, inventory contents or players.
    return {"id": item["id"], "case": case, "flavor": item["flavor"], "fixtureId": fixture["id"],
            "desktop": session["desktop"]["platform"], "agent": session["agent"]["platform"],
            "desktopHostSha256": hashlib.sha256(session["desktop"]["hostId"].encode()).hexdigest(),
            "agentRole": session["agent"]["role"], "agentHostSha256": hashlib.sha256(
                report["binding"]["agentHostId"].encode()).hexdigest(),
            "bindingSha256": hashlib.sha256(json.dumps(saved_scope["binding"], sort_keys=True).encode()).hexdigest(),
            "reportSha256": report_hash, "fixtureSha256": fixture_hash, "snapshotId": report["snapshotId"],
            "dimension": report["dimension"], "area": report["area"],
            "resourceGenerationId": report["resourceGenerationId"],
            "geometryGenerationId": report.get("geometryGenerationId"), "operationId": report["operationId"],
            "beforeOperationId": before_operation, "outcome": report.get("outcome"),
            "gameVersion": fixture["gameVersion"], "loaderVersion": fixture.get("loaderVersion"),
            "sourceSha256": source_hashes, "configSha256": fixture["configSha256"],
            "resourceSelectionSha256": fixture["resourceSelectionSha256"],
            "clientArtifactSha256": fixture["clientArtifactSha256"],
            "serverArtifactSha256": fixture["serverArtifactSha256"],
            "beforeCounts": before_counts, "afterCounts": counts(report), "artifacts": identities,
            "visual": visual_state, "observedAt": observed_at, "captures": captures, "bundle": bundle,
            "metrics": metrics, "metricsComplete": set(metrics) == set(METRICS),
            "disposition": disposition}


def collect(paths: list[Path], build_inputs: list[list[Path]]) -> dict:
    results, rejected, builds, seen = [], [], [], set()
    require(len(paths) <= 100, "session_limit")
    require(len(build_inputs) <= 100, "build_receipt_limit")
    for build_number, (receipt_path, jar_path) in enumerate(build_inputs, 1):
        try:
            receipt, receipt_hash = json_file(receipt_path)
            expected = receipt.get("prototypeJarSha256")
            require(isinstance(expected, str) and HASH.fullmatch(expected), "invalid_helper_build_identity")
            jar_identity = artifact(jar_path.parent, {"file": jar_path.name, "sha256": expected}, MAX_JSON)
            require(receipt.get("bytes") == jar_identity["bytes"], "helper_build_size_mismatch")
            require(receipt.get("launch", {}).get("gameLaunched") is False, "build_only_receipt_required")
            pins = receipt.get("pins", {})
            game, loader = pins.get("minecraft"), pins.get("forge", pins.get("neoforge"))
            require(isinstance(game, str) and ID.fullmatch(game) and
                    isinstance(loader, str) and ID.fullmatch(loader), "invalid_build_pins")
            builds.append({"receiptSha256": receipt_hash, "helper": jar_identity,
                           "gameVersion": game, "loaderVersion": loader,
                           "scope": "build_only", "visual": "pending"})
        except (ValueError, KeyError, TypeError, OSError, OverflowError, RecursionError) as error:
            code = str(error) if isinstance(error, EvidenceError) else "malformed_or_unavailable_build"
            rejected.append({"build": build_number, "code": code})
    for session_number, path in enumerate(paths, 1):
        try:
            session, _ = json_file(path)
            require(session.get("schemaVersion") == 1, "unknown_session_schema")
            require(session["desktop"]["platform"] in DESKTOPS and session["agent"]["platform"] in AGENTS and
                    session["agent"]["role"] in {"desktop", "headless", "service"}, "unsupported_platform")
            require(ID.fullmatch(session["agent"].get("hostId", "")) is not None, "invalid_agent_identity")
            require(ID.fullmatch(session["desktop"].get("hostId", "")) is not None, "invalid_desktop_identity")
            identities = {}
            for role in ("desktop", "agent"):
                ref = session["artifacts"][role]
                require(ref.get("platform") == session[role]["platform"] and ref.get("role") == role,
                        "package_platform_or_role_mismatch")
                identities[role] = artifact(path.parent, ref)
            items = session.get("observations")
            require(isinstance(items, list) and len(items) <= 1000, "observation_limit")
            for item_number, item in enumerate(items, 1):
                try:
                    result = observation(path.parent, item, session, identities)
                    require(result["id"] not in seen, "duplicate_observation_id")
                    seen.add(result["id"])
                    results.append(result)
                except (ValueError, KeyError, TypeError, OSError, OverflowError, RecursionError) as error:
                    code = str(error) if isinstance(error, EvidenceError) else "malformed_or_unavailable_evidence"
                    rejected.append({"session": session_number, "observation": item_number, "code": code})
        except (ValueError, KeyError, TypeError, OSError, OverflowError, RecursionError) as error:
            code = str(error) if isinstance(error, EvidenceError) else "malformed_or_unavailable_session"
            rejected.append({"session": session_number, "code": code})
    def cell(items: list[dict]) -> str:
        if not items:
            return "pending"
        # A later failure cannot disappear merely because an earlier receipt succeeded.
        if any(r["disposition"] == "failed" for r in items):
            return "failed"
        return "evidence_recorded" if any(r["disposition"] == "evidence_recorded" for r in items) else "pending_visual"
    matrix = [{"desktop": d, "agent": a, "state": cell([r for r in results if
               r["desktop"] == d and r["agent"] == a and r["case"] == "transport-import"])} for d, a in PAIRS]
    coverage = [{"desktop": d, "case": case, "state": cell([r for r in results if
                 r["desktop"] == d and r["case"] == case])}
                for d in DESKTOPS for case in FIXTURES + REPAIRS + CONTROLS + SCENARIOS
                if not case.startswith("native-windows-") or d == "windows-x86_64"]
    flavors = [{"flavor": f, "state": cell([r for r in results if r["flavor"] == f and
                r["case"] in FIXTURES])} for f in FLAVORS]
    costs = [{"desktop": d, "state": "measurements_recorded_budget_review_pending" if
              any(r["desktop"] == d and r["metricsComplete"] for r in results) else "pending"}
             for d in DESKTOPS]
    return {"schemaVersion": 1, "featureGate": "open_owner_and_independent_review_required",
            "transportMatrix": matrix, "platformCoverage": coverage, "flavors": flavors,
            "resourceCosts": costs,
            "observations": results, "builds": builds, "rejected": rejected,
            "note": "Evidence recorded is an owner observation, not an automated rendering pass. Missing production capture support and physical acceptance remain open."}


def markdown(result: dict) -> str:
    lines = ["# P18.21 collected evidence", "", "Feature gate: open; owner and independent review required.",
             "", "| Desktop | Agent | Transport/import evidence |", "|---|---|---|"]
    lines += [f"| {c['desktop']} | {c['agent']} | {c['state']} |" for c in result["transportMatrix"]]
    lines += ["", "| Desktop | Required case | Evidence |", "|---|---|---|"]
    lines += [f"| {c['desktop']} | {c['case']} | {c['state']} |" for c in result["platformCoverage"]]
    lines += ["", "| Java flavor | Evidence |", "|---|---|"]
    lines += [f"| {c['flavor']} | {c['state']} |" for c in result["flavors"]]
    lines += ["", "| Desktop | Measured resource cost |", "|---|---|"]
    lines += [f"| {c['desktop']} | {c['state']} |" for c in result["resourceCosts"]]
    lines += ["", f"Accepted observations: {len(result['observations'])}. Rejected inputs: {len(result['rejected'])}.",
              f"Build-only receipts: {len(result['builds'])}; these fill no rendering cells.",
              "", "Missing visual confirmations and platform results remain pending."]
    return "\n".join(lines) + "\n"


def confirmation(args: argparse.Namespace) -> dict:
    require(len(args.session) == 1 and args.owner_confirmed and
            args.map_capture is not None and args.minecraft_capture is not None,
            "explicit_owner_confirmation_and_two_captures_required")
    session, _ = json_file(args.session[0])
    items = [i for i in session.get("observations", []) if i.get("id") == args.confirmation_for]
    require(len(items) == 1 and items[0].get("native") is None, "one_java_observation_required")
    root = args.session[0].parent
    report, report_hash = referenced_json(root, items[0].get("report"))
    fixture, fixture_hash = referenced_json(root, items[0].get("fixture"))
    scope(report)
    require(fixture.get("snapshotId") == report["snapshotId"] and fixture.get("area") == report["area"] and
            fixture.get("dimension") == report["dimension"] and bool(fixture.get("namedBlocks")),
            "confirmation_saved_scope_mismatch")
    images = {}
    for role, filename in (("map", args.map_capture), ("minecraft", args.minecraft_capture)):
        path = checked_path(root, filename)
        before = path.stat()
        require(before.st_size <= 32 * 1024 * 1024, "capture_image_byte_limit")
        with path.open("rb") as stream:
            raw = stream.read(32 * 1024 * 1024 + 1)
        after = path.stat()
        require(0 < len(raw) <= 32 * 1024 * 1024 and len(raw) == before.st_size,
                "capture_image_byte_limit")
        require((before.st_size, before.st_mtime_ns, before.st_ino) ==
                (after.st_size, after.st_mtime_ns, after.st_ino), "capture_image_changed")
        require(raw.startswith(b"\x89PNG\r\n\x1a\n") or raw.startswith(b"\xff\xd8\xff") or
                raw[:4] == b"RIFF" and raw[8:12] == b"WEBP", "capture_image_required")
        images[role] = {"file": filename, "sha256": hashlib.sha256(raw).hexdigest()}
    return {"schemaVersion": 1, "confirmedBy": "Cameron", "confirmed": True,
            "observedAt": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "reportSha256": report_hash, "fixtureSha256": fixture_hash,
            "namedBlocks": fixture["namedBlocks"], "captures": images}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session", type=Path, action="append", default=[],
                        help="Private session manifest, repeat for each desktop/agent observation set.")
    parser.add_argument("--format", choices=("json", "markdown"), default="json")
    parser.add_argument("--build", type=Path, nargs=2, action="append", default=[],
                        metavar=("RECEIPT", "JAR"), help="Build-only helper receipt and its actual JAR; repeat as needed.")
    parser.add_argument("--confirmation-for", help="Print an owner confirmation receipt for this Java observation ID.")
    parser.add_argument("--owner-confirmed", action="store_true",
                        help="Owner assertion that the named appearances match; never inferred from a build.")
    parser.add_argument("--map-capture", help="Map screenshot relative to the session manifest.")
    parser.add_argument("--minecraft-capture", help="Minecraft screenshot relative to the session manifest.")
    args = parser.parse_args()
    try:
        if args.confirmation_for:
            print(json.dumps(confirmation(args), indent=2, allow_nan=False))
            return 0
        require(not args.owner_confirmed and args.map_capture is None and args.minecraft_capture is None,
                "confirmation_options_require_observation")
        result = collect(args.session, args.build)
        print(json.dumps(result, indent=2, allow_nan=False) if args.format == "json" else markdown(result), end="\n")
        # Pending is an honest report, not a failed test/release gate. Bad input is an error.
        return 2 if result["rejected"] else 0
    except (EvidenceError, OSError, ValueError, TypeError) as error:
        code = str(error) if isinstance(error, EvidenceError) else "collection_failed"
        print(code, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
