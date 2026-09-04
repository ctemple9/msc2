# Phase 12 physical beta release evidence

**Step:** P12.64 — Record the physical beta release gate  
**Status:** worksheet awaiting Cameron's physical-partition run  
**Scope:** x86_64 Ubuntu Server/Linux and x86_64 Windows

This packet records the part of the beta gate that CI cannot prove: a clean
machine installs the release, starts the agent without a graphical session,
keeps the service under the intended user, and remains usable when a client
closes or signs out. It is an evidence worksheet, not a claim that either
machine has passed yet.

The frozen artifact contract is in
[`phase12-release.md`](../phase12-release.md). The Linux and Windows
worksheets below are the hands-on records. `signing.md` records the expected
absence of distribution signing separately from checksum integrity.

## Start with the published bytes

Use one exact beta tag and download its six release assets plus `SHA256SUMS`
from the intended release page. Do not use a debug build, a candidate artifact
with a different version, or a file rebuilt locally.

Stage only the six installer/archive files in the local verifier directory and
keep the checksum file outside that directory:

```text
mkdir -p target/release/artifacts
cp <downloaded-six-assets> target/release/artifacts/
cp SHA256SUMS target/release/sha256sums.txt
python3 tools/release/verify-artifact-manifest.py \
  --manifest target/release/sha256sums.txt \
  --artifacts target/release/artifacts
```

The verifier requires one macOS, Windows, and Linux desktop asset and one
headless asset for each platform, all at x86_64 and all at the same release
version. It also rejects symlinks, directories, duplicate manifest entries,
missing assets, extra files, malformed hashes, and changed bytes. A successful
comparison proves downloaded bytes match the published bytes; it does not
prove that the release publisher was trusted.

Record the verifier output and the exact filenames in both platform
worksheets. Do not paste pairing codes, bearer tokens, passwords, private
addresses, or credential-store contents into this directory.

## Gate checklist

The gate remains open while any row is `Pending`. A green GitHub workflow is
necessary evidence for the release bytes, but it cannot replace these rows.

- [ ] The exact published asset set passes the verifier.
- [ ] [`linux.md`](linux.md) is complete for a clean physical x86_64 Ubuntu
      Server or qualifying Linux partition, including no desktop packages,
      boot start, tunnel access, pairing, remote Minecraft lifecycle, agent
      recovery, and logs.
- [ ] [`windows.md`](windows.md) is complete for a clean physical x86_64
      Windows partition, including installer launch, Tauri pairing/reconnect,
      service ownership, sign-out survival, and remote Minecraft lifecycle.
- [ ] [`signing.md`](signing.md) records the expected unsigned warnings and
      the fact that no signing/notarization claim is being made.
- [ ] The successful tag workflow run, release tag, artifact filenames, and
      manifest filename are recorded without turning CI into a hardware claim.

## Evidence hygiene

Each result must include the date, OS release, x86_64 architecture, exact
artifact filename and SHA-256, installing identity, service state, and the
observed result. Screenshots should show the relevant state without exposing
secrets. If a check cannot be performed, mark it `Unavailable`, explain why,
and leave the release gate open rather than converting it into a pass.
