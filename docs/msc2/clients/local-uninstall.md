# Complete local uninstall

Owner-requested 2026-10-02. P19.1 implements the shared read-only inventory;
P19.2 adds native discovery/removal; CLI and desktop entry points are being wired in P19.3–P19.4. No
uninstall entry point is available yet.

## Scope

Uninstall affects only the computer running MSC. Selecting a saved remote host
in the desktop does not change this boundary. Saved remote connections and
credentials on this device will be cleared, but no request is sent to a remote
agent. To uninstall another computer, run its local desktop or CLI there.

Remove verified MSC-owned services, helpers, servers/worlds/backups, host and
client state, credentials, logs/caches, installed command and desktop/package.
MSC 1, development checkouts, unrelated files, independently installed Java,
Tailscale/Docker, and OS package caches are excluded.

## Preview and authority

`msc-infrastructure::uninstall::inventory` consumes a `Request` assembled from
local native inspection. The function reads through the `FileSystem` interface
and has no deletion, service-control, HTTP, elevation, or process-launch code.
It returns serializable entries with identity, path, ownership evidence,
present/missing/blocked state, and a problem where inspection fails. It reads
configuration directly; it never uses configuration recovery that writes new
defaults over corrupt input.

The platform adapters supply locally inspected service reports, bundle/package
findings, native client-cache locations, and headless install roots. The shared
layer reads service path overrides from those reports rather than trusting the
selected host or treating the current shell environment as the service's
configuration. Service definitions without usable inspection block removal.
A caller that omits desktop/package inspection receives a blocked finding.
P19.2 supplies the actual OS inspection adapters; P19.1 does not claim to
identify every installed package on its own.

Only the three path overrides are copied from service environments. Preview
serialization never includes the full environment, authentication tokens, or
secret contents. Windows credential cleanup is represented by the MSC2 target
prefix for the current user; native credential enumeration/deletion is P19.2.

The shared inventory covers platform data-directory conventions and explicit
service paths, configured roots and registered server directories, bundle-ID
client state, marked headless install directories, and verified selected
installers. A configured server root is an entire deletion tree, including any
worlds/backups inside it, not just the currently selected server. Any external
server directory appears separately with its registry evidence and all of the
same protection checks. Native adapters must identify additional external
backup/helper locations before claiming complete cleanup.

Targets must be absolute, without traversal, and may not resolve through
symlinks or overlap root/home/shared parents, MSC 1, supplied protected paths,
or detected developer checkouts. A blocked child also blocks its parent:
removing the parent would otherwise defeat the child's protection. Missing
files are separate from unreadable ones. Corrupt configuration blocks deletion
of the data tree containing it. Existing custom data roots without an MSC
configuration are ambiguous and blocked. Native findings undergo the same
path validation before they appear in the preview.

Each preview has a deterministic SHA-256 fingerprint. This is a review
comparison, not a security token, proof of ownership, or permission to delete.
The executor must obtain fresh local inspection, compare the preview, enforce
confirmation, and validate ownership and filesystem boundaries again just
before deletion. OS service and package operations need separate native
identity validation; an arbitrary serialized `Entry` is never executable
cleanup instructions.

## Confirmation and completion (planned)

Desktop: preview the exact local computer/targets, acknowledge permanent loss
of worlds and backups, type `UNINSTALL MSC 2`, then confirm in a final dialog.
The native command enforces confirmation independently of the frontend.

CLI: `msc uninstall --danger` shows the same preview and asks for that exact
phrase. `msc uninstall --dry-run` changes nothing. Explicit automation uses
`--confirm "UNINSTALL MSC 2"`; redirected input without it is refused. No
uninstall route is added to the HTTP API. Existing Reset remains unchanged.

The native executor must first gracefully stop Minecraft/helpers, remove
services, clean data/credentials, and use verified OS installation mechanisms
for app removal. Self-removal may need a protected detached continuation.
Scheduled work is not reported as completed uninstall. Failures must show what
remains and preserve a readable result outside the deleted MSC paths.

## Installers

A filename such as MSC_2.dmg is not ownership evidence. A candidate must have a
SHA-256 checksum obtained from verified signed MSC release metadata, and the
current bytes must match it. Changed or unverified candidates are blocked.
Unknown renamed/moved installers cannot be discovered reliably and must be
selected and verified explicitly. Mounted-image unmount and native package
identity handling are execution work. Never directly purge OS package caches.

## Essential boundary checks

The six in-memory cases in `crates/msc-infrastructure/tests/uninstall.rs`
protect against concrete destructive failures: shared/protected/source/symlink
targets, deletion of a parent containing corrupt configuration, ambiguous
custom-directory ownership, leaking service secrets or ignoring its path
overrides, deleting an unrelated or changed installer, and preview writes.
They use only controlled fake files, have no network/services/sleeps, and are
expected to complete in under one second together once compiled. They were
added but not run; only Cameron can authorize a test command.

Non-test verification for P19.1:

```sh
cargo check -p msc-infrastructure
```
