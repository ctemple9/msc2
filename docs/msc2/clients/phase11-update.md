# Phase 11 coordinated-update policy

This policy covers MSC application releases only: the desktop shell, its
bundled agent and required sidecar, and the independently installed headless
agent. It does not update Minecraft server jars, worlds, loaders, add-ons,
modpacks, or plugins.

## Release source and trust decision

GitHub Releases is the transport and release-note source. The client fetches
release metadata and notes over HTTPS from the configured MSC repository, but
the GitHub API response, tag name, release name, release body, and checksum
file are discovery and integrity data only. None of them authorizes an update.

An update is eligible only when all of these hold:

1. The release supplies one canonical JSON manifest and a detached Ed25519
   signature over that manifest's UTF-8 canonical bytes.
2. The signature verifies against the public key packaged with the local MSC
   installation (`MSC2_RELEASE_PUBLIC_KEY_HEX`). The private signing key is a
   GitHub Actions secret and never enters this repository or a shipped binary.
3. The manifest's release ID, API compatibility range, platform, architecture,
   exact asset set, byte sizes, and SHA-256 digests match the local request.
4. The current client API major and minor fall within the manifest's inclusive
   compatibility range. A different API major or an out-of-range minor is
   refused; the manifest does not override D-010's version-skew rules.

The manifest contract is deliberately small and machine-checkable. Its
required fields are:

| Field | Meaning |
|---|---|
| `schemaVersion` | Version of this signed manifest shape. Unknown major shapes are refused. |
| `releaseId` | Immutable release identifier, without a `latest` alias. |
| `tag` | Exact immutable Git tag associated with the release. |
| `api` | `{ major, minMinor, maxMinor }`, with an inclusive minor range. |
| `platforms` | Entries keyed by supported platform/architecture, each with an install mode and exact assets. |
| `platforms[*].assets` | Objects containing `role`, exact `filename`, positive `bytes`, and lowercase 64-character `sha256`. |

Asset URLs are derived from the configured GitHub repository, release ID, and
the signed filenames. An unsigned API URL is never followed, and a manifest
cannot redirect the updater to an arbitrary host.

## What updates together

An MSC desktop release is a **release set**, not three independent update
buttons. On macOS it contains the desktop installer, agent, and the
compatible Swift Bedrock sidecar. On Windows it contains the desktop
installer and agent; native Bedrock is part of the agent package rather than a
sidecar. The manifest names the exact file and SHA-256 digest for every member.
The sidecar is required on macOS and forbidden on Windows.

The first signed manifest may also describe the Linux desktop package or a
standalone headless archive. Those are separate platform entries, not a reason
to make one Linux artifact pretend to be every installation type.

## Bounded retrieval and staging

Manifest, signature, release-note, per-asset, and total-download limits are
fixed before retrieval. HTTPS requests reject oversized responses, missing or
duplicate assets, unexpected filenames, redirects outside the configured
GitHub repository, and incomplete downloads. The updater downloads only the
selected signed assets; it never mirrors a release or trusts an unbounded
archive listing.

The native shell downloads the manifest and selected artifacts into a new,
release-ID-named staging directory beneath the local agent data directory's
`updates/` folder. It verifies the signature, compatibility range, platform,
architecture, asset roles, filenames, byte sizes, and digests before the set
is considered staged. An interrupted, cancelled, invalid, or superseded
staging directory is discardable and cannot become active.

Staging never modifies configuration, secret-store records, worlds, server
files, the installed service definition, or the running agent.

## Explicit approval and recovery

Checking and staging are non-installing operations. The client shows the
release ID, release notes, exact asset set, and installation mode, then asks
for a separate explicit confirmation for that same staged release ID. There
is no timer, background approval, or automatic restart path. Declining or
cancelling leaves the current installation untouched.

The installer records the previous release before replacement and keeps it
available until the replacement has passed its local health check. If
replacement or health recovery fails, it restores the previous release and
reports a rollback. A failed installation is never reported as successful.
Configuration, secrets, worlds, and server directories remain at their
existing paths throughout the update and rollback sequence.

## Installation modes and privilege boundary

Updates are always local to the computer being updated. A remote Tauri client
may check or update its own desktop installation, but an authenticated API
connection can never install, start, stop, replace, or uninstall the
operating-system service on another host. `server stop` remains Minecraft
process control, not service management.

| Installation | Update action |
|---|---|
| macOS or Windows Tauri desktop | After local confirmation, launch the exact verified coordinated installer. Replace the desktop, bundled agent, and macOS sidecar together where applicable. Stop/restart the local agent only within that verified replacement and health-recovery sequence. |
| Linux Tauri desktop `.deb` or `.rpm` | After local confirmation, hand the exact verified package to an authorized local package-install operation. The updater does not overwrite package-owned files directly and does not manage a remote host's service. |
| Standalone headless archive on macOS, Windows, or Linux | After local CLI authorization, replace only the verified local binary/resources from the matching archive, using the same previous-release, service-restart, health-check, and rollback rules. The local service definition is changed only through the local installer privilege boundary. |
| Distribution-managed Linux installation | Report the release ID, package name, and package-manager action. The distribution package manager remains the owner of installation; MSC does not silently replace files or create a second updater. |

The desktop shell, browser, and CLI must report when the selected host is
remote or when the current installation is distribution-managed. None may
turn a remote management request into a privileged host-service operation.

## Scope boundaries

The release manifest cannot contain Minecraft server, loader, component,
add-on, modpack, or plugin updates. Those affect a managed server and remain
in their own operation-backed workflows. A checksum comparison can establish
that bytes match a published file; only the signed coordinated manifest makes
an MSC application update eligible.
