# Bedrock VM appliance resources

The distributable sidecar bundle must contain the Intel (`x86_64`) appliance
pair from the MSC 1 VZ backend:

- `vmlinuz-kata`
- `appliance-initramfs.gz`

The sidecar checks both resources before accepting `provision`. The build input
is explicit: set `MSC2_BEDROCK_APPLIANCE_DIR` to a directory containing these
two files. For example:

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

The Xcode validation phase fails before compilation when the directory is
unset, either file is missing, or a checksum differs. The binaries stay out of
this source commit because they are generated distribution artifacts, not
source code. The desktop staging script copies the validated pair beside
`BedrockSidecar` and the agent; a headless macOS package uses the same sibling
layout. Linux and Windows keep their native BDS archive as a verified first-run
download and do not receive this VM appliance.

Apple Silicon is deliberately rejected before booting: Phase 10 ships only the
Intel appliance and does not add an arm64 guest or Rosetta-for-Linux path.
