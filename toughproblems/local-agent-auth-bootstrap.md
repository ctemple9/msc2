# The local agent wouldn't connect (P12.2 review, 2026-08-25)

**Symptom:** Cameron opened the real Tauri app to visually verify P12.2
(Overview tab) and got "Agent unavailable" / "Agent starting" / "Action
needed" no matter what he clicked. Reconnect, Start, and Repair all
appeared to do nothing. This took roughly a dozen rounds to fully resolve
and touched five separate, unrelated bugs stacked on top of each other —
worth a permanent record so nobody re-discovers them the hard way.

**Why this doc exists:** none of this was caused by the P12.2 Overview
work itself. It surfaced because that was the first time anyone actually
exercised the real Tauri app's connection-to-agent path end to end. The
underlying auth-bootstrap feature had been built (see "Where the fix came
from" below) but never actually gotten working.

## The six real bugs, in the order they were found

Each of these produced a different, genuine error — this was not one bug
misdiagnosed six times. Fixing one always revealed the next one underneath.

### 1. Missing CORS on `GET /v1/health`

**Symptom:** Tauri dev window stuck on "Agent starting" even though `curl`
confirmed the agent was healthy.

**Cause:** In `tauri dev`, the app's webview loads its UI from the Vite
dev server (`http://127.0.0.1:1420`) — a different origin than the agent
(`http://127.0.0.1:48001`). The pre-credential readiness probe
(`localAgentHealthCheck` in `clients/desktop-web/src/lib/platform/index.ts`)
used the browser's plain `fetch()`, and the agent sent no
`Access-Control-Allow-Origin` header at all, so the browser silently
blocked the response.

**Fix:** `crates/msc-agent/src/routes/health.rs` now sets
`Access-Control-Allow-Origin: *` on this one route. It's already
documented as running outside the bearer-auth gate and carries no secrets,
so a permissive CORS allowance here doesn't weaken anything. Committed
separately as `P11.30`.

*(Later made moot for the Tauri path specifically — the WIP work below
added `agent_health_check`, a native Tauri command using `reqwest`
instead of the webview's `fetch()`, which bypasses browser CORS entirely.
The header fix is still what makes a plain **browser** dev session work,
so it stayed.)*

### 2. The LaunchDaemon privilege gap ("Start agent" did nothing)

**Symptom:** Clicking "Start agent" in the UI produced no visible effect
and no error.

**Cause:** The agent is registered as a **root-owned system LaunchDaemon**
(`/Library/LaunchDaemons/com.ctemple.msc2.agent.plist`). The Tauri app
runs as a normal user and was calling plain `launchctl start <label>`
unprivileged — which can't control a system-domain daemon it doesn't own.
The failure was swallowed (an unhandled promise rejection with no UI
surface), so it silently did nothing.

**Fix:** this is where we discovered `git stash@{0}` — "WIP: local-agent
auto-bootstrap auth (pre-P12.1)" — an already-fairly-complete
implementation Cameron had built and set aside before Phase 12 started.
It included `install_and_start_elevated`/`start_elevated` in
`crates/msc-platform-macos/src/service.rs`, which run
`osascript -e 'do shell script "..." with administrator privileges'` to
get a real macOS password prompt before touching the LaunchDaemon. We
applied that stash rather than rewriting the feature from scratch.

### 3. Self-signing a running process invalidates its own code identity

**Symptom, after fixing #2:** the password prompt appeared, but the agent
log showed `bootstrap peer failed code-identity validation: code identity
has been invalidated`.

**Cause:** The stash's design (see "Where the fix came from") verifies
the connecting desktop process's macOS code-signing identity before
trusting it. A `cargo`/`tauri dev` build is never actually signed (only
`tauri build`'s bundling step signs for real), so the very first
`desktop_code_requirement()` call found no signature and ad-hoc-signed the
binary on the spot (`codesign --force --sign -`) — while that exact file
was the one the *currently running* process had been `exec`'d from.
Rewriting your own backing executable while you're running from it is
exactly the kind of tampering code-signing enforcement exists to catch;
macOS marks that process's code identity invalidated from then on.

**Fix:** moved the self-signing check to app startup, in
`ensure_ad_hoc_signed_or_reexec()` in
`clients/desktop-web/src-tauri/src/lib.rs`. If the binary is unsigned, it
signs it and then **re-execs itself** (`Command::new(&executable).exec()`,
replacing the process image outright) instead of continuing to run as the
now-poisoned process. The process that continues past that call was
loaded fresh from the already-signed file and has a valid identity.

**A sharp edge this created:** every `tauri dev` relaunch appears to
relink the binary (even with no source changes), which changes its
content and therefore its ad-hoc signature's cdhash. The installed
plist's `MSC2_MACOS_DESKTOP_REQUIREMENT` is a snapshot of *some previous*
run's cdhash, so it goes stale on every restart. **Practical rule: click
"Repair service" once, right after the app you're about to test opens —
and don't restart the app again afterward before you test.** A "Repair"
done before the app's most recent restart will not match.

### 4. Keychain ACL didn't trust the agent binary to read its own secrets

**Symptom, after fixing #3:** code identity now validated, but
`reading bootstrap installation key: reading installation-key-v1: User
interaction is not allowed.`, and later the same error for
`credential-root-v1`.

**Cause:** the stash's design stored two secrets in the **System**
keychain (`/Library/Keychains/System.keychain`, chosen because it's
readable without an interactive login session — a LaunchDaemon has none):
an "installation key" for the challenge-response proof, and a "root key"
that's the master encryption key for every other secret the agent stores.
Items written via `security add-generic-password` with no `-T` flag
default their ACL to trusting only whatever process created them (`security`
itself). The agent — a non-interactive daemon with no session to answer
an "allow access?" prompt — was never on that trust list, so every read
failed closed instead of prompting.

**First fix attempt:** add `-T <path-to-agent-binary>` when writing both
items, and delete-then-recreate them on every install/repair (since items
created *before* this fix existed would otherwise keep their stale ACL
forever, and no "repair" could ever fix that in place without also
patching the ACL of an existing item, which needs the keychain's own
password). This got further, but running as root under `osascript`'s
elevation still isn't the same trust context as an interactive session in
every case, and it kept recurring in slightly different forms. After
several rounds of this, Cameron asked to drop keychain entirely rather
than keep chasing ACL edge cases.

### Why we removed keychain entirely

The agent and its desktop shell **always run as the same regular macOS
user**. That means the plain Unix file-permission boundary that already
protects every other secret this store keeps (each one is already a 0600
file under `~/Library/Application Support/MSC 2/secrets/`, individually
AEAD-encrypted) is exactly as strong a boundary for these two secrets too
— without any of the keychain ACL/session semantics that kept failing in
practice. Keychain earns its keep when a secret must survive being read by
a *different* user or a fully separate untrusted process; that's not this
case.

**What changed** (`crates/msc-platform-macos/src/secret_store.rs`,
`crates/msc-platform-macos/src/service.rs`,
`crates/msc-agent/src/auth/local_bootstrap.rs`,
`clients/desktop-web/src-tauri/src/lib.rs`):

- The store's root key moved from a System-keychain item to
  `<secrets_dir>/.root-key` — a 0600 file, self-provisioned (generated on
  first use) by the agent itself. No install-time provisioning step
  needed at all any more.
- The installation key moved from a System-keychain item to
  `<secrets_dir>/local-bootstrap.key` — also 0600, written directly by the
  **desktop app**, unprivileged, before it ever calls the elevated install.
  The elevated install script no longer touches either secret; it only
  checks the key file already exists, then does the plist/launchctl work.
- `install_and_start_elevated`'s shell script lost every `security
  add-generic-password`/`delete-generic-password`/`find-generic-password`
  call. It's just: bootout old daemon (if any), install the plist,
  bootstrap, kickstart.

**Trade-off accepted:** resetting `.root-key` (which the old keychain-based
"repair" already effectively did by regenerating a keychain item with a
different ACL) discards every secret it protects, since they're all
encrypted under it. Not a concern yet — nothing production-meaningful is
stored there during this bootstrap work. A future "repair" that needs to
preserve accumulated secrets would need to detect *why* an existing root
key is unreadable rather than always replacing it — not implemented, since
today the file's a normal Unix permission this account already owns, so
"unreadable" shouldn't normally happen at all.

**One resulting manual cleanup, done once:** rotating the root key left
one already-encrypted secret (`remote-api.agent-host-id`) undecryptable —
`decrypting remote-api.agent-host-id: aead::Error`. Deleted the one stale
file under `secrets/`; the agent regenerated it cleanly on next use.

### 5. `hostId` vs `host_id` — a genuine serde bug

**Symptom, after fixing #4:** code identity validated, installation key
read fine, but `bootstrap proof is invalid` — a JSON *parse* failure, not
a proof mismatch.

**Cause:** `ClientProof` in
`crates/msc-agent/src/auth/local_bootstrap.rs` (the struct the agent
deserializes the desktop app's proof submission into) was missing
`#[serde(rename_all = "camelCase")]`, unlike its sibling response structs
in the same file. The agent expected literal JSON key `host_id`; the
desktop app's `bootstrap_local_macos()` in
`clients/desktop-web/src-tauri/src/lib.rs` sends `hostId` (matching every
other message in the same protocol). Every proof submission failed to
parse before the agent ever got to check whether the proof itself was
correct.

**Fix:** one line — add the missing `#[serde(rename_all = "camelCase")]`
to `ClientProof`.

## Where the fix came from: `git stash@{0}`

Partway through debugging bug #2, a `git log --all --grep=bootstrap`
turned up `git stash@{0}`: **"On main: WIP: local-agent auto-bootstrap
auth (pre-P12.1)"** — a substantial, mostly-working implementation of
exactly this feature that Cameron had built and stashed before Phase 12
began, and which had sat untouched since. It already had the hard parts
(macOS code-signing identity verification via `security-framework`'s
`SecCode`, the `osascript` elevation prompt, the challenge-response
protocol over a Unix-domain socket) — it just had never been fully
exercised end-to-end. We applied it (`git stash apply stash@{0}`, clean —
only `App.svelte` had a trivial, non-conflicting overlap with unrelated
P12.2 work) and fixed forward from there rather than re-implementing.

**Lesson:** `git stash list` is worth checking before building an
"impossible" feature from scratch. This one had already been half-solved.

## The debugging technique that actually worked

Every round here followed the same loop, and it's what eventually cut
through the fog:

1. Ask Cameron to click the thing.
2. Immediately `tail` the real agent log
   (`~/Library/Application Support/MSC 2/logs/agent.log`) for the *exact*
   new error text — not a paraphrase.
3. Grep the actual source for that exact string to find precisely which
   check produced it, rather than guessing from the symptom alone.
4. Fix only that one thing, rebuild, and copy the rebuilt agent binary
   into `clients/desktop-web/src-tauri/target/Resources/agent/msc` — the
   path the installed LaunchDaemon actually launches, which is **not**
   the same file `cargo build` at the workspace root produces, and does
   **not** get updated by rebuilding the Tauri app.

Two gotchas that cost real time:

- **The agent log is append-only and never rotated.** Old errors from
  earlier attempts sit right above the newest ones with no separator.
  Always check the process start time (`ps -o lstart`) against the log
  entries you're reading, or you'll diagnose a bug that was already fixed.
- **Signing a file changes its identity for any process already running
  from it.** Don't `codesign` a binary that's currently backing a live
  process (bug #3 above) — either re-sign before first launch, or re-exec
  after signing.
