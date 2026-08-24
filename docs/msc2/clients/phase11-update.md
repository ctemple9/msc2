# Phase 11 coordinated-update policy

## What updates together

An MSC desktop release is a **release set**, not three independent update
buttons. On macOS it contains the signed desktop installer, agent, and the
compatible Swift Bedrock sidecar. On Windows it contains the signed desktop
installer and agent; native Bedrock is part of the agent package rather than a
sidecar. The manifest names the exact file and SHA-256 digest for every
member, and is signed by the MSC release key embedded in the desktop package at
build time. Packaging provides only the public key
(`MSC2_RELEASE_PUBLIC_KEY_HEX`); the private signing key never enters this
repository or the installed application.

The native shell downloads the manifest and artifacts into a new,
release-ID-named staging directory. It verifies the manifest signature,
platform, compatibility range, artifact set, file names, and digests before
the set is considered staged. Staging writes only beneath the agent data
directory's `updates/` folder. It never modifies configuration, secret-store
records, worlds, server files, or the running agent.

The desktop installer is the platform-signed installer in the staged set. A
macOS installer replaces the app bundle, packaged agent, and sidecar together;
a Windows installer replaces the desktop and agent together. The installer
stops the agent only after its replacement has been verified, runs a local
post-install health check, and restores the recorded previous release if that
check fails. It does not copy user data as part of replacement: configuration,
secrets, worlds, and server directories remain at their existing paths.

## Explicit approval and recovery

Checking and staging are non-installing operations. The shell shows the
release ID and exact set members, then requires an explicit confirmation for
that same staged release ID before it launches the platform installer. There
is no timer, background approval, or automatic restart path. A cancelled,
missing, changed, unsigned, or digest-mismatched release remains uninstalled.

The installer records the previous release before replacement. If replacement
or its health check fails, it restores that release and leaves the agent's data
untouched. A failed installation is reported as a rollback, never as a
successful update.

## Compatibility and scope boundaries

Each manifest advertises one API major and an inclusive API-minor range. The
desktop API minor must fall within that range before staging; this implements
D-010's approved supported-floor-and-degradation mechanism without choosing
the still-proposed N-3 number. A manifest with a different API major is
refused. The release set may include a sidecar only when its platform requires
one; macOS requires the sidecar and Windows must not pretend to have one.

Linux never runs this updater. The client may report a release as available,
but directs the person to the distribution package manager with the release ID
and package name. This preserves headless package ownership and avoids a
second Linux updater.

This policy is deliberately separate from Minecraft server, loader, component,
modpack, and add-on updates. Those affect a server and remain in their own
managed workflows; they cannot be placed in a desktop release manifest.
