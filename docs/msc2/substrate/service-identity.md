# Service identity and privilege boundaries for v1 (D-025)

**Status: Confirmed** by Cameron Temple, 2026-08-01 — questions 1, 2, 3, and 6 (the installing-user identity model and the install-time-only escalation boundary), and the recommended macOS default of §3 (design P3.9 against the System keychain now, rather than block on a live LaunchDaemon test). Question 4's macOS sub-case and question 5 (TCC) stay genuinely **Open** — Cameron's confirmation picks the default to build against, it does not resolve the underlying, untestable-until-Phase-4 platform question of whether a `UserName`-scoped LaunchDaemon can actually reach the login Keychain. `msc2-decisions.md` is amended accordingly.

---

## 1. What D-025 asks, and why it blocks this phase

`msc2-decisions.md` D-025 opened six questions the audit corpus has no answer for, because MSC 1 has never faced them: it is a user-session GUI application, so its files, its Keychain items, and its processes all belong to whoever is running it. A macOS LaunchDaemon, a Windows Service, and a `systemd` service all run **outside the logged-in user's session** — a genuinely new problem, not something portable from Swift.

This blocks Phase 3 directly: P3.9–P3.11 build the real `SecretStore` implementations, and question 4 (machine-scoped secret storage) decides what those implementations target. This step answers as far as reading the platform's own documented behavior responsibly allows, and says plainly where that runs out.

## 2. Recommended direction: the agent runs as the account that installed it

On all three platforms, by default:

| Platform | Mechanism |
|---|---|
| **macOS** | LaunchDaemon plist's `UserName` key set to the installing user (not `root`) |
| **Windows** | Service "Log on as" set to that same user (not `LocalSystem`) |
| **Linux** | `systemd` unit's `User=`/`Group=` set to that same user (not a dedicated system account) |

The installer captures the identity of the account running the installer (`$LOGNAME` / `whoami` equivalent on macOS and Linux, the invoking user's SID on Windows) and writes it into the daemon/service/unit definition at install time. No separate service account is created.

This is a real behavior change from every off-the-shelf "run it as a system service" tutorial, which defaults to `root`/`SYSTEM`/a dedicated account precisely because those tutorials assume no single human owns the machine. D-011's own rationale is the opposite case: *"the owner already runs MSC on an always-on spare Mac managed mostly from iOS"* — one person, one machine, one account. Optimizing for the dedicated-multi-admin case here would be solving a problem this product doesn't have yet, at the cost of the case it does have.

### Why this answers questions 1–3 by construction

**Q1 — which OS account runs the agent?** The installing user. Not `root`/`SYSTEM`, not a dedicated service account.

**Q2 — who owns server directories?** The same account, because the process that creates them is that account. A desktop user opening, editing, or backing up a world folder in Finder/Explorer/a file manager needs no special group membership, no ACL grant, no `sudo` — it's already their own file, the same as it is in MSC 1 today. This is the concrete, user-visible payoff of the recommendation: the alternative (a dedicated service account) would silently reintroduce exactly the kind of ownership friction D-025 exists to avoid, for a multi-admin scenario this product doesn't target in v1.

**Q3 — when is privilege escalation permitted?** Routine operation needs none — the agent already runs as an unprivileged user account for every server-management action. The one place escalation is unavoidable is **writing the daemon/service/unit definition itself at install time**:

- macOS: `launchd` refuses to load a `LaunchDaemon` plist unless `/Library/LaunchDaemons/<id>.plist` is root-owned and not group/other-writable — even though the `UserName` key inside it points at an unprivileged account, the file placement step needs root.
- Windows: registering a service with the Service Control Manager requires the `SERVICE_ALL_ACCESS` / admin-equivalent right regardless of which account the service later logs on as.
- Linux: writing a unit file into `/etc/systemd/system/` and running `systemctl enable` needs root, even though the unit's own `User=` line names an unprivileged account.

All three are gated by the OS's own installer-elevation prompt (`sudo`/admin password / UAC), the same one-time elevation MSC 1 already asks for today when it installs anything system-level. No new escalation surface is introduced; it's confined to install time, not routine operation.

**Q6 — how do updates cross the privilege boundary?** Two cases, not one:

- **Updating the agent binary itself.** If installed to a location the installing user already owns (e.g. under their home directory or a user-writable app-support path), a self-update needs no elevation — it's the same account overwriting its own file. If installed to a system-protected location (`/Library/PrivilegedHelperTools`-equivalent, `Program Files`, `/usr/local/`), it needs the same one-time elevation prompt install did. Which of these applies is an install-location decision this step doesn't make — flagged for whoever designs the self-update mechanism, not resolved here.
- **Updating the daemon/service/unit definition itself** (e.g. changing which account it runs as, or its restart policy) needs the same root/admin path as initial installation, for the same reason: the definition file's own permissions require it.

Routine content updates (new server versions, new mods, config changes) touch only files the agent's own unprivileged account already owns — no escalation, no daemon-definition change, nothing crosses the boundary at all.

## 3. Question 4 — machine-scoped secret storage — split into a resolved half and an open half

**Windows and Linux are answered.** DPAPI's user-scope mode is reachable by whichever account the service logs on as — since that's now the installing user (§2), DPAPI's per-user protection applies exactly as it would for a normal desktop application, with no daemon/session mismatch to reason about. `systemd-creds`, once P3.2 confirms it as the Linux backend, is designed to work with a normal (non-`DynamicUser=`) unit running as a real user — P3.2 covers this in its own dedicated step, referenced here rather than duplicated.

**macOS is not answered — genuinely Open, not guessed.** The specific sub-question: does a `LaunchDaemon` with `UserName` set to a real user actually gain access to that user's unlocked **login** Keychain?

What can be said from documented platform behavior, short of a live test:

- A `LaunchDaemon` runs outside any `loginwindow` security session. `UserName` changes the Unix identity (UID/GID) the process runs under; it does not, by itself, attach the process to the user's login session or the keychain-unlock state tied to that session.
- Login Keychain access is conventionally scoped to processes running *inside* a user's GUI/security session — which is exactly the boundary D-025 exists because a headless LaunchDaemon crosses.
- This is widely reported, consistent practitioner experience among macOS systems programmers, but it is not something this audit corpus — built from MSC 1's own source, which has never run as a LaunchDaemon — can verify. Guessing an answer here would produce exactly the "decision with no evidence behind it" D-025 itself warns against.

**Consequence for this phase:** P3.9 (the macOS `SecretStore` implementation) cannot responsibly choose between targeting the login keychain and targeting the **System** keychain (`SecKeychain` system domain, not tied to any login session, reachable by any locally-running process including daemons) until this is tested against an actual `LaunchDaemon` — which doesn't exist until Phase 4 builds real service registration.

**Confirmed by Cameron Temple, 2026-08-01: design `SecretStore`'s macOS implementation against the System keychain now**, rather than block P3.9 on a live LaunchDaemon test that can't happen until Phase 4. This unblocks P3.9 — it does not close the underlying platform question, which stays Open. Per-item access-control lists restrict readability to the agent's own process; document plainly (per §8's own requirement, "what it does and does not protect against") that this is a machine-scoped secret, not a user-scoped one — recoverable by anything running as root on the same machine. Unlike DPAPI, which is user-scoped, not machine-scoped (see the corrected comparison in `secret-storage.md` §13) — closer in category to `systemd-creds`'s host-key fallback without per-user isolation (`secret-storage.md` §2, §12). If Phase 4's live LaunchDaemon test shows login-keychain access does work with `UserName` set, that's a strictly better outcome and can be adopted then without having blocked this phase on it.

## 4. Question 5 — TCC — recorded as unverifiable from docs, deferred to Phase 4

Whether a macOS LaunchDaemon touching a user's Documents folder or an external volume triggers a TCC consent prompt — and what UI surfaces that prompt when there is no GUI session for the daemon to present it in — is genuinely undocumented in the audit corpus and not something reading MSC 1's source (a GUI app that has never crossed this boundary) can answer. Left as a known unknown, to be tested once a LaunchDaemon actually exists (Phase 4), exactly as D-025 originally scoped it.

## 5. Deferred, not required now: dedicated-service-account mode

For a genuine multi-admin dedicated host with no single "owning" desktop user, running the agent as a dedicated service account (with explicit group-based ownership handoff for server directories) is a real, coherent alternative. It is **not** built in v1 — D-011's own rationale is a single-owner machine, and building the group/ACL machinery a dedicated account requires, for a scenario this product doesn't yet target, would be exactly the kind of premature generality this project's own conventions warn against. Recorded here as a named **v1.1 option**, not silently foreclosed.

## 6. Phase 4 executable macOS check

P4.4 adds `tools/phase4/macos-launchdaemon-check.sh`, a live check for the two macOS questions this document deliberately left open in Phase 3. It installs a short-lived LaunchDaemon with `UserName` set to the installing user, runs a one-shot worker under that daemon identity, records the result, then unloads and removes the test daemon.

The worker checks three things:

- Login keychain behavior: `security add-generic-password`, `find-generic-password`, and `delete-generic-password` against the installing user's `~/Library/Keychains/login.keychain-db` path, falling back to `login.keychain` on older systems.
- System keychain behavior: the same add/find/delete sequence against `/Library/Keychains/System.keychain`.
- TCC behavior: create, write, read, delete, and remove a deliberately chosen test directory, such as a directory under Documents or on an external volume.

The script's dry run is the P4.4 verification command. It prints the planned plist path, daemon label, keychain paths, TCC directory, and cleanup actions without installing anything. The real run is intentionally still explicit:

```text
sudo tools/phase4/macos-launchdaemon-check.sh --tcc-dir "$HOME/Documents/MSC2LaunchDaemonTccCheck"
```

Observed result as of P4.4: dry-run planning works; the live keychain/TCC answer is still pending until Cameron or the later macOS service step runs the real command on the target machine. The production macOS `SecretStore` default therefore remains the System keychain answer confirmed in §3; this check exists to replace that conservative default only if the live daemon evidence justifies it.

**Production macOS credential write path (P4.40, 2026-08-12):** no new live
LaunchDaemon keychain result was captured in this review session because the
real check stopped at the local `sudo` password prompt. The earlier P3.9/P4.4
evidence therefore still controls: unprivileged routine writes to the System
keychain are not assumed to work, login-keychain reachability from a
`UserName`-scoped LaunchDaemon remains open, and the owner-confirmed production
target remains the System keychain until contrary daemon evidence exists. The
implementation recommendation is to keep System-keychain use in the privileged
install/update window and make routine service operation use a durable
agent-owned encrypted credential store protected by that provisioned
System-keychain material, unless a later live daemon run proves direct
routine Keychain mutation is reliable. That recommendation is an implementation
amendment path; it does not silently change the approved installing-user service
identity or the confirmed System-keychain target.

## 7. Summary — status of each of D-025's six questions after this step

| # | Question | Status |
|---|---|---|
| 1 | Which OS account runs the agent? | **Confirmed** (Cameron Temple, 2026-08-01) — the installing user, all three platforms |
| 2 | Who owns server directories? | **Confirmed** — same account, by construction |
| 3 | When is privilege escalation permitted? | **Confirmed** — install-time daemon/service/unit registration only |
| 4 | Machine-scoped secret storage | **Confirmed** for Windows/Linux (DPAPI user-scope, `systemd-creds` — see P3.2) · macOS default **confirmed as System keychain** to unblock P3.9 — underlying login-vs-System-keychain reachability from a `UserName`-scoped LaunchDaemon now has a P4.4 executable check, live result pending |
| 5 | How does a desktop user grant file access (TCC)? | **Open** — P4.4 executable check added, live result pending |
| 6 | How do updates cross the privilege boundary? | **Confirmed** — binary updates follow install location; daemon/service/unit-definition updates need the same elevation installation did |
