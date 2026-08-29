---
id: handbook.how-bedrock-runs
kind: handbook
title: How Bedrock Runs
category: bedrock-servers
subtitle: "How the app runs Bedrock Dedicated Server without Docker or any external software."
analogy: "Mojang only made BDS for Linux — there's no Mac version. MSC bundles a tiny lightweight Linux virtual machine that runs inside your Mac. The BDS server thinks it's running on a Linux machine, which it is — just one that lives invisibly inside the app."
relatedIds: [handbook.bedrock, bedrock.runtime-unavailable]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: dockerContent}
---

**Why a VM?** Mojang has never released a native macOS binary for Bedrock Dedicated Server. The official BDS binary is Linux-only. MSC bundles a minimal Linux VM (using Apple's built-in Virtualization framework) that runs the Linux BDS binary transparently. No Docker, no external downloads, no extra installs required.

### Callout: tip

The built-in VM starts and stops automatically with the server. You never need to open or interact with any external tool — just click Start and Stop like any other server type.

### In This App

- BDS is downloaded automatically on first start and cached in your server folder.
- VM start/stop is wired to the Start/Stop buttons — same UI as Java servers.
- World data is stored in your server folder and shared with the VM. Your data is never locked inside the VM image.
- Console output streams from the VM in real-time, just like Java.
- Updating BDS: use the version selector in the Components tab — the app downloads and installs the new version automatically.

### Advanced Details

Under the hood, MSC uses Apple's Virtualization.framework to boot a compact Linux guest (a custom minimal kernel + initramfs, ~11 MB bundled with the app). The BDS binary lives in your server directory and is shared into the VM via virtio-fs — no image to pull, no layer cache.

Bedrock UDP port 19132 is forwarded from the host into the VM via a tiny UDP relay so LAN clients and Playit.gg tunnels reach the server transparently. Commands typed in the console go to BDS stdin over the VM serial console.
