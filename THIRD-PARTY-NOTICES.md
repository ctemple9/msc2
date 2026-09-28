# Third-party notices

Apache-2.0 in [`LICENSE`](LICENSE) covers MSC 2's original code and
contributions. It does not relicense third-party dependencies, Minecraft
server software, or bundled runtime material. Each component keeps its own
copyright and license terms.

## Bundled Intel macOS Bedrock appliance

The Intel macOS desktop and headless distributions include
`sidecar/bedrock/Resources/vmlinuz-kata` and
`sidecar/bedrock/Resources/appliance-initramfs.gz`. Their recorded SHA-256
values and packaging boundary are in
[`sidecar/bedrock/Resources/README.md`](sidecar/bedrock/Resources/README.md).
The kernel image identifies itself as Linux 6.18.35; Linux kernel source is
licensed GPL-2.0. The initramfs contains BusyBox 1.35.0, which is distributed
under GPL version 2 only, and Debian glibc 2.36-9+deb12u14. See the [Linux
kernel licensing rules](https://docs.kernel.org/process/license-rules.html),
the [BusyBox 1.35.0 source release](https://busybox.net/downloads/busybox-1.35.0.tar.bz2),
and [Debian's glibc copyright and source records](https://sources.debian.org/src/glibc/2.36-9%2Bdeb12u14/).

These appliance files were carried forward as distribution artifacts from
MSC 1. This repository does not contain the kernel's exact source revision,
Kata build configuration, the complete initramfs package manifest, or
corresponding source and notices for every included binary and certificate
bundle. The existing hashes establish file identity only; they do not establish
license compliance or grant redistribution rights. The project license does
not cover these files. Recover and publish the matching source, package
notices, and build provenance before treating a new public distribution of
these appliance files as cleared.

The tracked `sidecar/bedrock/Resources/init` script is MSC 2 original code and
is covered by Apache-2.0. The compressed initramfs also contains system
libraries and certificate data that are not independently tracked as source.

## Downloaded server software and integrations

MSC 2 can download or interact with software maintained by other projects,
including Paper, Purpur, Fabric, NeoForge, MinecraftForge, Geyser, Floodgate,
Chunker, Xboxbroadcast, Adoptium Temurin, and the Minecraft Java and Bedrock
server distributions. Those projects retain their own terms. MSC 2's mention
or integration does not grant rights to their software or trademarks. Obtain
each download from its documented provider and follow that provider's terms.

The Cargo and npm lockfiles identify the selected Rust and frontend dependency
versions. Their licenses remain those declared by their authors. A per-release
dependency and license inventory is planned in P16.22; the lockfiles alone are
not a substitute for third-party license notices.
