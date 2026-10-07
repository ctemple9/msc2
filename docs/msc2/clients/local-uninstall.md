# Complete local uninstall

Owner-requested 2026-10-02. Implemented in P19.1–P19.4; destructive acceptance
on disposable installed environments remains with Cameron.

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
prefix for the current user; native credential enumeration/deletion is implemented in P19.2.

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

## Confirmation and completion

Desktop: preview the exact local computer/targets, acknowledge permanent loss
of worlds and backups, type `UNINSTALL MSC 2`, then confirm in a final dialog.
The native command enforces confirmation independently of the frontend.

CLI: `msc uninstall --danger` shows the same preview and asks for that exact
phrase. `msc uninstall --dry-run` changes nothing. Explicit automation uses
`--confirm "UNINSTALL MSC 2"`; redirected input without it is refused. No
uninstall route is added to the HTTP API. Existing Reset remains unchanged.

The local worker gracefully stops Minecraft before the native executor removes
services/helpers, cleans data/credentials, and uses verified OS installation mechanisms
for app removal through a protected detached continuation.
On Linux, the installed root-owned MSC helper stays open through those privileged
steps so the OS asks for authorization once. It accepts only fixed service,
system-data, and verified installation actions; the worker still removes
user-owned files after checking the reviewed inventory.
Scheduled work is not reported as completed uninstall. Failures must show what
remains and preserve a readable result outside the deleted MSC paths.

## Installers

A filename such as MSC_2.dmg is not ownership evidence. A candidate must have a
SHA-256 checksum obtained from verified signed MSC release metadata, and the
current bytes must match it. Changed or unverified candidates are blocked.
Moved installers must be selected explicitly and retain a filename found in
the verified staged catalog. Older installers without retained signed metadata
are left in place. Mounted images and OS package caches are not purged.

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

## Desktop entry point

Open the app settings (gear), scroll to **Reset and uninstall**, and choose
**Uninstall…**. The separate sheet shows native targets and exclusions, permits
verified installer selection, and requires the permanent-loss checkbox, exact
phrase, and final dialog. A removal report in the home folder is enabled by
default. Native handoff succeeds before the desktop closes. The chosen remote
host is never passed to the local command.

## CLI entry point

```sh
msc uninstall --dry-run
msc uninstall --danger
```

The first command is read-only. The second displays the inventory, warns about
world/backup loss, and requires the exact `UNINSTALL MSC 2` phrase. Automation
must supply `--danger --confirm "UNINSTALL MSC 2"`; use `--fingerprint` from the
reviewed JSON preview to refuse changed targets. `--installer /absolute/path`
adds a candidate that must match staged signed release metadata. `--report
/absolute/new-result.json` explicitly retains a result outside removed trees.

Removal is handed to a copied private worker. A scheduled response means only
that the worker was launched. It waits for the parent and desktop to close,
rediscovers targets, verifies the preview, stops the local Minecraft server,
and removes services/data/installation. Failed or partial removal keeps a
report and exits nonzero in the worker; the parent cannot report that later
result as its own synchronous exit code. Interactive terminals receive worker
output. Desktop reads the scheduled report location before closing.

## Windows package repair and ordinary removal (P16.48)

Reopen the installed MSI or use Windows' application maintenance entry to choose
Repair or Remove. Repair restores this version's package files and registration;
it retains settings, worlds, account/password and agent boot policy. The owned
agent follows the same graceful Minecraft shutdown as an update and resumes only
if it was running beforehand. Minecraft itself remains stopped. Same-product
repair can prove a legacy copied build against the helper's compiled hashes even
when the installed agent files need restoration.

Ordinary Remove deletes package-owned app/tools, shortcuts and PATH registration.
It gracefully stops and disables only a proven package-owned service while file
removal runs, then detaches that service at commit. It keeps worlds, servers,
backups, settings, saved credentials and copied agent recovery builds. It never
runs complete data cleanup and never offers Launch on completion. Independent
headless services are retained. Major-upgrade child removal skips service actions.
After ordinary removal, reinstall and set up local hosting again, including the
Windows account password. A service pending deletion may require closing Services
or restarting Windows before registering it again.

Before commit, rollback retains the original SCM registration/password and
restores metadata, owner record, boot policy (including delayed automatic start)
and previous agent state. Detachment is the final fallible native operation:
Windows cannot return the service's stored password after deletion. A failure in
later Windows Installer commit work can therefore leave a restored app with no
service; the recovery error/log requires reinstall and local hosting setup. A
retained transaction record blocks another owned-service package transaction and
must be inspected, never discarded automatically.

The separately confirmed full-removal worker removes its inspected service and
data first. Its fixed Windows adapter validates the protected DesktopLifecycle
cache, checksum-named builds and exact four-file payload before removing those
files and empty directories. Unknown files, unfinished transactions, redirected
paths or an existing service refuse cleanup. It never follows data paths from the
ownership record. MSI subsequently sees no service and performs package-only
removal. Only that confirmed worker clears credentials/worlds; ordinary MSI does
not invoke it.
