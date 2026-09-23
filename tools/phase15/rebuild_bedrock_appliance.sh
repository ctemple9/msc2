#!/bin/sh
set -eu

workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
resource_root="$workspace_root/sidecar/bedrock/Resources"
output_path="$resource_root/appliance-initramfs.gz"
staging_root=$(mktemp -d "${TMPDIR:-/tmp}/msc2-bedrock-initrd.XXXXXX")

gzip -dc "$output_path" | (cd "$staging_root" && cpio -idm >/dev/null 2>&1)
cp "$resource_root/init" "$staging_root/init"
chmod 755 "$staging_root/init"
(cd "$staging_root" && find . -print | cpio -o -H newc 2>/dev/null) | gzip -9 > "$output_path"
shasum -a 256 "$output_path"
