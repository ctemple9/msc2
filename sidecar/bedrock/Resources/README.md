# Bedrock VM appliance resources

The distributable sidecar bundle must contain the Intel (`x86_64`) appliance
pair from the MSC 1 VZ backend:

- `vmlinuz-kata`
- `appliance-initramfs.gz`

The sidecar checks both resources before accepting `provision`. The verified
pair is checked into this MSC 2 repository in this directory, so the desktop
development build uses it automatically. Set `MSC2_BEDROCK_APPLIANCE_DIR` only
when deliberately testing or packaging a replacement directory. For example:

```sh
xcodebuild -project sidecar/bedrock/BedrockSidecar.xcodeproj \
  -scheme BedrockSidecar MSC2_BEDROCK_APPLIANCE_DIR=/path/to/intel-appliance
```

The recorded SHA-256 identities are the MSC 1 distribution artifacts used by
this port:

| File | SHA-256 |
|---|---|
| `vmlinuz-kata` | `85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e` |
| `appliance-initramfs.gz` | `0865eb432f61249a5a2f76770e7c79e53cf803c5fa435d110ced03747da8a278` |

The Xcode validation phase fails before compilation when either file is missing
or a checksum differs. The pair is a verified distribution input rather than
source code, but it is intentionally tracked here so MSC 2 has no build-time
dependency on the MSC 1 checkout. The desktop staging script copies the
validated pair beside `BedrockSidecar` and the agent; a headless macOS package
uses the same sibling layout. Linux and Windows do not receive this VM appliance
and do not read this directory; they keep their native BDS archive as a
verified first-run download.

Apple Silicon is deliberately rejected before booting: Phase 10 ships only the
Intel appliance and does not add an arm64 guest or Rosetta-for-Linux path.
