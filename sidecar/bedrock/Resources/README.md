# Bedrock VM appliance resources

The distributable sidecar bundle must contain the Intel (`x86_64`) appliance
pair from the MSC 1 VZ backend:

- `vmlinuz-kata`
- `appliance-initramfs.gz`

The sidecar checks both resources before accepting `provision`. They are kept
out of this source commit because they are generated binary distribution
artifacts, not source code. Apple Silicon is deliberately rejected before
booting: Phase 10 ships only the Intel appliance and does not add an arm64
guest or Rosetta-for-Linux path.
