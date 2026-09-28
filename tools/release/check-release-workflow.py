#!/usr/bin/env python3
"""Check the artifact-only cross-platform beta workflow contract."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class WorkflowError(Exception):
    """A human-readable release workflow failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise WorkflowError(message)


def read_workflow(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise WorkflowError(f"cannot read workflow: {error}") from error


def check_yaml(path: Path) -> None:
    """Use an installed parser when available, with a Ruby fallback on CI."""
    try:
        import yaml  # type: ignore[import-not-found]
    except ImportError:
        result = subprocess.run(
            [
                "ruby",
                "-e",
                "require 'yaml'; YAML.load_file(ARGV.fetch(0))",
                str(path),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        require(result.returncode == 0, f"workflow is not valid YAML: {result.stderr.strip()}")
    else:
        try:
            yaml.safe_load(path.read_text(encoding="utf-8"))
        except yaml.YAMLError as error:
            raise WorkflowError(f"workflow is not valid YAML: {error}") from error


def require_fragment(workflow: str, fragment: str) -> None:
    require(fragment in workflow, f"workflow is missing {fragment!r}")


def check_candidate_workflow(workflow: str) -> None:
    for fragment in (
        "workflow_dispatch:",
        "push:",
        "tags:",
        "- 'v*'",
        "jobs:",
        "runs-on: ${{ matrix.os }}",
        "fail-fast: false",
        "macos-15-intel",
        "macos-14",
        "windows-latest",
        "ubuntu-22.04",
        "toolchain: 1.97.1",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "npm run check",
        "npm run api:check",
        "npm run test:contract",
        "npm run test:auth-desktop",
        "npm run test:tauri-boundary",
        "npm run test:fedora-regressions",
        "cargo fmt --all -- --check",
        "cargo --locked test -p msc-agent --bin msc routes::components::staged_upload_tests::chunked_modpack_upload_requires_order_and_completes_with_verified_size -- --exact",
        "cargo --locked test -p msc-agent --bin msc routes::components::staged_upload_tests::world_import_chunks_can_be_cancelled_and_removed_idempotently -- --exact",
        "cargo --locked test -p msc-agent --bin msc cli::update::tests::authorized_update_waits_for_authorizer_and_reports_cancellation -- --exact",
        "cargo --locked build --release",
        "--bundles \"${{ matrix.tauri-bundles }}\" --no-sign",
        "tauri-bundles: dmg",
        "tauri-bundles: msi",
        "tauri-bundles: deb,rpm",
        "patchelf \\",
        "rpm",
        "build-linux-headless.sh",
        "check-linux-artifacts.py",
        "--max-glibc 2.36",
        "debian:12-slim",
        "fedora:44",
        "build-macos-headless.sh",
        "build-windows-headless.ps1",
        "prepare-windows-icon.py",
        "ci-evidence:",
        "require-ci-run.py",
        "actions: read",
        "Record release builder environment",
        "BUILD-ENVIRONMENT.json",
        "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
        "beta-${{ matrix.platform }}-${{ env.RELEASE_VERSION }}",
        "UNSIGNED-BETA-NOTICE.txt",
        "notarization",
    ):
        require_fragment(workflow, fragment)

    for obsolete in (
        "bundle:stage-agent",
        "bundle:check-agent",
        "check-client-bundle.py",
        "web-ui",
        "--test web_ui",
    ):
        require(obsolete not in workflow, f"workflow still depends on retired browser bundle item {obsolete!r}")

    action_refs = re.findall(r"(?m)^\s*uses:\s+[^\s@]+@([^\s#]+)", workflow)
    require(action_refs, "workflow has no GitHub Actions references")
    require(
        all(re.fullmatch(r"[0-9a-f]{40}", reference) for reference in action_refs),
        "every third-party action must be pinned to a full commit SHA",
    )
    require("node-version: '22.23.1'" in workflow, "release Node.js version is not exact")
    require("toolchain: 1.97.1" in workflow, "release Rust version is not exact")
    require("tool: cargo-nextest@0.9.143" in workflow, "cargo-nextest version is not exact")

    require(
        re.search(r"cargo --locked clippy -p msc-agent --bin msc --target", workflow) is not None,
        "workflow is missing the targeted msc-agent binary clippy check",
    )
    require(
        '"bundle":{"icon":["icons/icon.ico"]}' in workflow,
        "Windows Tauri build does not select the prepared ICO resource",
    )

    require(
        re.search(r"platform:\s+macos-x86_64", workflow) is not None,
        "macOS artifact must be labelled x86_64",
    )
    require(
        re.search(r"platform:\s+macos-aarch64", workflow) is not None,
        "Apple Silicon macOS artifact must be labelled aarch64",
    )
    require(
        re.search(r"platform:\s+windows-x86_64", workflow) is not None,
        "Windows artifact must be labelled x86_64",
    )
    require(
        re.search(r"platform:\s+linux-x86_64", workflow) is not None,
        "Linux artifact must be labelled x86_64",
    )
    require("gh release" not in workflow, "candidate workflow must not publish a GitHub release")
    require("softprops/action-gh-release" not in workflow, "candidate workflow must not publish a GitHub release")
    require("notarytool" not in workflow, "candidate workflow must not invoke notarization")
    require("signtool" not in workflow, "candidate workflow must not invoke Authenticode signing")
    require("APPLE_CERTIFICATE" not in workflow, "candidate workflow must not load a signing certificate")


def check_publish_guard(workflow: str) -> None:
    require(re.search(r"(?m)^\s+publish:", workflow) is not None, "publish job is missing")
    publish_start = re.search(r"(?m)^  publish:\s*$", workflow)
    require(publish_start is not None, "publish job is not a top-level job")
    publish_job = workflow[publish_start.start() :]
    require("needs: [build, ci-evidence]" in publish_job, "publish job does not require build and CI evidence")
    require("needs.build.result == 'success'" in publish_job, "publish job does not require a successful matrix")
    require("needs.ci-evidence.result == 'success'" in publish_job, "publish job does not require successful same-commit CI")
    require("github.event_name == 'push'" in publish_job, "publish job is not push-guarded")
    require("github.event_name == 'workflow_dispatch'" in publish_job, "publish job lacks manual-dispatch handling")
    require("inputs.publish == true" in publish_job, "manual publication is not explicitly opted in")
    require("startsWith(github.ref, 'refs/tags/v')" in publish_job, "publish job is not tag-guarded")
    require("permissions:" in publish_job and "contents: write" in publish_job, "publish job lacks release permission")
    require("actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093" in publish_job, "publish job does not collect matrix artifacts")
    require("verify-artifact-manifest.py" in workflow, "manifest verifier is not wired")
    require("--write" in publish_job and "SHA256SUMS" in publish_job, "SHA-256 manifest generation is not wired")
    require("sign-update-manifest.py" in publish_job, "signed update manifest publisher is not wired")
    require("packaging/update-release-schema.json" in workflow, "update manifest schema is not referenced")
    require(
        "MSC2_RELEASE_SIGNING_KEY_HEX: ${{ secrets.MSC2_RELEASE_SIGNING_KEY_HEX }}" in publish_job,
        "publish job does not read the release signing key from GitHub Actions secrets",
    )
    require("msc2-update-manifest.json" in publish_job, "publish job does not require the signed manifest")
    require("msc2-update-manifest.sig" in publish_job, "publish job does not require the manifest signature")
    require("RELEASE-NOTES.md" in publish_job, "publish job does not publish release notes")
    require("MSC2_RELEASE_PUBLIC_KEY_HEX: ${{ vars.MSC2_RELEASE_PUBLIC_KEY_HEX }}" in publish_job, "publish job does not read the release public key")
    require("--public-key-env MSC2_RELEASE_PUBLIC_KEY_HEX" in publish_job, "manifest signing does not verify the public/private key pair")
    require("MSC2_RELEASE_SIGNING_KEY_HEX must be configured" in publish_job, "publish job allows unsigned releases")
    require("-name '*.rpm'" in publish_job, "publication does not collect RPM assets")
    require('test "$asset_count" -eq 9' in publish_job, "publication does not require nine release assets")
    require("softprops/action-gh-release@da05d552573ad5aba039eaac05058a918a7bf631" in publish_job, "publish job does not create a GitHub release")
    require("tools/release/check-provenance.py" in publish_job, "publish job does not generate and verify dependency provenance")
    require("BUILD-ENVIRONMENT-" in publish_job, "publish job does not preserve platform builder records")
    require("actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020" in publish_job, "publish job does not pin Node.js for provenance")
    require("dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de" in publish_job, "publish job does not pin Rust for provenance")
    require("prerelease: true" in publish_job, "GitHub publication is not marked as a prerelease")
    require("fail_on_unmatched_files: true" in publish_job, "release publication does not fail on missing assets")
    require("CI-EVIDENCE.json" in publish_job, "release publication does not attach exact workflow run evidence")
    ci_waiter = read_workflow(ROOT / "tools/release/require-ci-run.py")
    for fragment in (
        '"event": "push"',
        '"head_sha": sha',
        '"head_branch": tag',
        '"status") != "completed"',
        '"conclusion") != "success"',
        'job.get("status") != "completed"',
        'job.get("conclusion") != "success"',
        "REQUIRED_JOB_COUNTS",
    ):
        require(fragment in ci_waiter, f"same-commit CI gate is missing {fragment!r}")
    ci_workflow = read_workflow(ROOT / ".github/workflows/ci.yml")
    ci_action_refs = re.findall(r"(?m)^\s*uses:\s+[^\s@]+@([^\s#]+)", ci_workflow)
    require(ci_action_refs, "CI workflow has no GitHub Actions references")
    require(
        all(re.fullmatch(r"[0-9a-f]{40}", reference) for reference in ci_action_refs),
        "every CI action must be pinned to a full commit SHA",
    )
    require('node-version: "22.23.1"' in ci_workflow, "CI Node.js version is not exact")
    require('channel = "1.97.1"' in (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"), "repository Rust toolchain is not exact")
    require("cargo-nextest@0.9.143" in ci_workflow, "CI cargo-nextest version is not exact")
    require("tags: ['v*']" in ci_workflow, "CI does not run the full workflow on version tags")
    require("npm run api:check" in ci_workflow, "CI does not check generated API types")
    require(
        "github.event_name != 'workflow_dispatch'" in ci_workflow,
        "tag CI can still skip required jobs through a focused dispatch scope",
    )
    manifest_checker = read_workflow(ROOT / "tools/release/verify-artifact-manifest.py")
    require('"CI-EVIDENCE.json"' in manifest_checker, "release checksum gate does not treat CI evidence as metadata")
    signer = read_workflow(ROOT / "tools/release/sign-update-manifest.py")
    require(
        "verify_signature_with_openssl(bytes.fromhex(public_value), manifest_bytes, signature)" in signer,
        "manifest signer does not independently verify the Ed25519 signature",
    )


def split_publish_job(workflow: str) -> tuple[str, str]:
    """Return the candidate workflow and the optional publication job."""
    match = re.search(r"(?m)^  publish:\s*$", workflow)
    if match is None:
        return workflow, ""
    return workflow[: match.start()], workflow[match.start() :]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("workflow", type=Path)
    parser.add_argument(
        "--expect-publish-guard",
        action="store_true",
        help="also require the guarded publication job used by the next release step",
    )
    args = parser.parse_args()
    path = args.workflow if args.workflow.is_absolute() else ROOT / args.workflow
    try:
        workflow = read_workflow(path)
        check_yaml(path)
        candidate_workflow, _ = split_publish_job(workflow)
        check_candidate_workflow(candidate_workflow)
        if args.expect_publish_guard:
            check_publish_guard(workflow)
    except WorkflowError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    message = "cross-platform artifact candidate is valid"
    if args.expect_publish_guard:
        message += " with guarded publication"
    print(f"OK: {message}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
