# Bedrock VM appliance resources

The distributable sidecar bundle must contain the Intel (`x86_64`) appliance
pair from the MSC 1 VZ backend. The initramfs also carries glibc compatibility
links for the current official BDS binary's legacy `libpthread`, `libdl`, and
`librt` sonames:

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

The kernel is the MSC 1 distribution input. The initramfs includes MSC 2's
guest startup script; its SHA-256 changes whenever that script is rebuilt:

| File | SHA-256 |
|---|---|
| `vmlinuz-kata` | `85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e` |
| `appliance-initramfs.gz` | `fbbaaefd80aa47e18343367ac171babb3cdcecf7da58acc14ac94db4f77bcce6` |

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

The checked-in `init` source controls the guest process identity. Rebuild the
compressed initramfs after changing it with
`sh tools/phase15/rebuild_bedrock_appliance.sh`, then update the checksum in
the Xcode project and this file before packaging. The root helper supervises a
VM worker whose real and effective UID/GID match the installing account.
Apple's shared-folder writes use that host identity; guest numeric IDs alone
do not control host ownership. The worker also supplies `msc_uid`/`msc_gid`
kernel arguments and runs BDS under those guest IDs. Invalid IDs stop startup
instead of falling back to root. Network relays stay in the root supervisor.
