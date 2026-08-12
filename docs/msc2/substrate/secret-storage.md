# Secret storage: Linux backend decision (P3.2) and the `SecretStore` trait (P3.8)

**Status: Confirmed** by Cameron Temple, 2026-08-01 — `systemd-creds` as the Linux backend, and (resolving §5's flagged constraint) **Debian 12 "bookworm" as MSC 2's Linux minimum**, over building the root-owned-file fallback to also cover Debian 11 and older. `msc2-decisions.md` D-011 is amended accordingly.

**Sections 1–7 below are P3.2's Linux-backend decision.** Sections 8 onward are P3.8's extension: the cross-platform `SecretStore` trait every platform implementation (P3.9 macOS, P3.10 Windows, P3.11 this Linux backend) builds against, its key-naming scheme, and the macOS Keychain-scope answer P3.9 needs before it can start.

---

## 1. The choice: `systemd` credentials (`systemd-creds` / `LoadCredential=`)

`msc2-engineering.md` §8 named three candidates and already flagged one as its "preferred candidate":

| Option | Notes (from §8) |
|---|---|
| **`systemd` credentials** (`LoadCredential=` / `systemd-creds`) | Encrypted at rest against the TPM where available; integrates with the service manager MSC already requires. Preferred candidate. |
| Root-owned file with restrictive permissions | Simple, universal, no dependencies. Weaker at rest; acceptable only with clear documentation of the threat model. |
| Secret Service when present, fallback when absent | Best experience on desktop Linux, but two code paths and two threat models to reason about. |

**This step confirms that preference in writing.** Two reasons, both already implicit in §8 but not yet stated as a decision:

1. **D-011 already commits Linux to a `systemd` unit with zero desktop dependencies.** `systemd-creds` is not a new dependency — it's the same service manager the agent already requires to run at all, providing a security feature for free rather than pulling in `gnome-keyring`/KWallet, which a minimal Debian install doesn't have.
2. **One code path, not two.** §8 already names the risk directly: "two code paths and two threat models to reason about." Since headless minimal Debian is a primary deployment target (D-011) and desktop Linux is not the primary target, there is no case in v1 that actually needs the Secret-Service branch. Building it anyway would be exactly the premature generality this project's conventions warn against — a second `SecretStore` implementation with its own failure modes, protecting a scenario nothing in the port plan currently asks for.

The root-owned-file option is not chosen, but is worth keeping in view: it is the natural degrade-to path if `systemd-creds` turns out to be unavailable on some supported target (see §5). It is not built as an automatic runtime fallback in v1 — see §6.

## 2. What `systemd-creds` does and does not protect against at rest

Per §8's own requirement — "the `SecretStore` trait must state what it does and does not protect against on each platform" — stated plainly, not left implicit:

`systemd-creds encrypt` (and the unit-level `LoadCredentialEncrypted=`/`SetCredentialEncrypted=` directives built on it) has two modes, selected automatically by what the host has available:

- **TPM2-sealed** (`--with-key=tpm2` or `auto` when a TPM2 chip is present): the encryption key is sealed to the TPM, optionally bound to PCR measurements of the boot state. The encrypted blob is only decryptable **on this specific machine**, and optionally only in an unmodified boot state. Copying the encrypted credential file to another machine — or exfiltrating the disk image — does not yield the secret; the TPM itself must be present and asked to unseal it.
- **Host-key fallback** (`--with-key=host`, used automatically when no TPM2 is present): the encryption key is a per-machine secret stored at `/var/lib/systemd/credentials.secret`, root-owned, mode `0600`. This is weaker: anyone who gains root on the live machine, or obtains both the encrypted credential *and* that key file (e.g. from a raw disk image), can decrypt. It protects against a stolen backup or a copied config directory read by an unprivileged process — not against local root compromise.

**Either way, this is a machine-scoped secret, not a user-scoped one** — the same category as the System-keychain default P3.1 just confirmed for macOS. Windows is not in this category: Credential Manager wraps DPAPI's *per-user* mode, tied to the installing account rather than to the whole machine (P3.10) — an earlier draft of this document mistakenly called DPAPI machine-scoped too; §13's cross-platform table has the corrected comparison. Consequence for this platform and macOS: anything running as root on the same machine can recover the secret (via the TPM, if present, or via the host key file). See §13 for the full three-platform comparison, including the respect in which Linux's actual v1 backend (§12) ends up weaker than the other two.

At the point of use, the decrypted value is exposed to the service process via a `CREDENTIALS_DIRECTORY` — a `tmpfs`-backed, root-owned directory (mode `0700`) containing one file per credential, each mode `0400` and owned by the UID the unit actually runs as. Only that process (and root) can read it. It is never written to a world- or group-readable path, and it does not appear in the unit file itself, in `systemctl show`, or in the process environment where a `ps`/`/proc` inspection could catch it.

## 3. Interaction with P3.1's installing-user identity

P3.1 (`docs/msc2/substrate/service-identity.md`) confirmed the agent's Linux identity is the installing user via `User=`/`Group=` in the unit — explicitly **not** a dedicated account, and **not** `systemd`'s `DynamicUser=` mechanism.

This is compatible with `systemd-creds` without adjustment. `LoadCredential=`/`LoadCredentialEncrypted=` scope the decrypted `CREDENTIALS_DIRECTORY` to whichever UID the unit's `User=` names — dynamic or static, it makes no difference to the credential-loading mechanism itself. `DynamicUser=` is commonly paired with `systemd-creds` in examples because both are systemd-native hardening features that show up together in write-ups, not because one requires the other. P3.1's design (a real, persistent installing-user account) works with `LoadCredentialEncrypted=` exactly as a `DynamicUser=`-allocated ephemeral account would.

## 4. Provisioning: who runs `systemd-creds encrypt`, and when

Sealing a secret with `systemd-creds encrypt` needs to happen before the unit can load it, and the encrypted output is conventionally placed alongside the unit file or referenced by absolute path in `LoadCredentialEncrypted=<name>:<path>`. Writing to either location needs root — the same install-time elevation boundary P3.1 already established for writing the unit file itself (`/etc/systemd/system/` requires root regardless of which unprivileged user the unit later runs as).

Concretely: the installer, already running elevated to write the `systemd` unit (P3.1 §2), also runs `systemd-creds encrypt` for any secret that needs to be provisioned at install time (e.g. a pairing token, per D-012's Phase 3 scope note about wiring real credential storage into pairing). This introduces no new escalation surface — it's one more root-owned write during the same install-time elevation window, not a separate privileged operation at a different time. Secrets created *after* install (e.g. a re-paired token) go through the same path: `msc-agent` cannot self-elevate to reseal a credential, so either re-provisioning a secret requires re-running an elevated helper, or the chosen backend must support an unprivileged write path. **This is a real open question this step surfaces but does not resolve** — flagged for whoever implements P3.11 (the Linux `SecretStore` implementation), not answered here, because it depends on implementation details (a small root-run helper binary vs. a `systemd-creds`-adjacent mechanism) this planning step shouldn't guess at.

## 5. Constraint to flag: minimum `systemd` version

`LoadCredentialEncrypted=`/`SetCredentialEncrypted=` require **`systemd` 250 or later** (unencrypted `LoadCredential=` is older, but the encrypted form this step relies on for at-rest protection is not). This matters concretely against D-011's "minimal Debian" target, which doesn't pin an exact Debian release:

| Debian release | `systemd` version | `LoadCredentialEncrypted=` available? |
|---|---|---|
| 12 "bookworm" (current stable) | 252 | Yes |
| 11 "bullseye" (oldstable) | 247 | **No** |

**Confirmed by Cameron Temple, 2026-08-01: MSC 2's Linux minimum is Debian 12 "bookworm"** (or any distribution with `systemd` ≥ 250), over building the root-owned-file fallback to also cover Debian 11 and older. This closes the question this step could not decide unilaterally — bookworm has been current stable since mid-2023, and is what a fresh minimal-Debian install performed today would produce, matching D-011's own framing. `msc2-decisions.md` D-011 is amended with this floor. The root-owned-file fallback (§8's own table) is therefore **not** built as a runtime path in v1 — see §6.

## 6. What this does not build

Per §6 of the Phase 3 intro's "not in this phase" list and this step's own scope: this step is a decision, not an implementation. P3.11 builds the actual `SecretStore for Linux` implementation against this choice. Not built now, and not silently assumed for later either:

- **The root-owned-file fallback**, as an automatic runtime degrade path if `systemd-creds` is unavailable. Not built unless §5's minimum-Debian-version question comes back requiring it.
- **The Secret-Service branch**, for desktop Linux. Named as a real v1.1-shaped option in the same spirit as P3.1's deferred dedicated-service-account mode, not built now — no current target in the port plan needs it.
- **The re-provisioning helper** named in §4 as an open question — left for P3.11 to design, not guessed at here.

## 7. Summary

| Question | Answer |
|---|---|
| Which of §8's three candidates is chosen? | **`systemd` credentials** (`systemd-creds` / `LoadCredentialEncrypted=`) — confirms §8's own stated preference |
| What does it protect against at rest? | TPM2-sealed when available (machine- and optionally boot-state-bound); host-key fallback otherwise (root-on-this-machine can decrypt) |
| What does it *not* protect against? | Anything running as root on the same live machine — a machine-scoped secret, same category as the macOS System-keychain default (D-025); Windows DPAPI is user-scoped, not machine-scoped, so it is not in this category (see §13) |
| Does it require `DynamicUser=`? | No — works identically with P3.1's static installing-user `User=`/`Group=` |
| Who runs `systemd-creds encrypt`, and when? | The installer, during the same elevated install-time window that already writes the unit file (P3.1) |
| Minimum `systemd` version? | 250+, for the *encrypted* credential directives. **Confirmed: Debian 12 (bookworm) is MSC 2's Linux floor** — ships 252, qualifies. Debian 11 (bullseye) ships 247 and is not supported. |
| Is the Secret-Service branch built? | No — one code path only, per §8's own stated reason to avoid two threat models |

---

## 8. The `SecretStore` trait

MSC 1's `KeychainManager.swift` hardcodes five read/write/delete pairs, one method-set per secret kind (`readRemoteAPIToken`/`writeRemoteAPIToken`, `readRemoteAPIGuestToken`/`writeRemoteAPIGuestToken`, `readXboxBroadcastAltPassword(forServerId:)`/`writeXboxBroadcastAltPassword(_:forServerId:)`, `readPlayitSecretKey`/`writePlayitSecretKey`, `readCurseForgeAPIKey`/`writeCurseForgeAPIKey`, lines 53–132) sitting on top of three generic Keychain primitives keyed by a `(service, account)` pair (`read`/`write`/`delete`, lines 162–228). Every new secret MSC 1 ever needed a new pair of typed methods for.

`msc-infrastructure::secret_store` generalizes this to one trait, keyed by a single string instead of a `(service, account)` pair, so a new secret kind is a new key, not a new method:

```rust
pub trait SecretStore {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}
```

Behavior generalized directly from `KeychainManager`'s own primitives, not invented:

- `get` on a key that was never set returns `Ok(None)`, not an error — `read`'s own `guard status == errSecSuccess ... else return nil` folds `errSecItemNotFound` into `nil`, not a thrown error (line 162).
- `set` is an upsert — `write`'s own doc comment states this plainly: "Upsert: updates if the item exists, adds if it does not" (line 184).
- `delete` on a key that was never set is `Ok(())`, not an error — `delete`'s own comment: "`errSecItemNotFound` is acceptable — the item was already absent" (line 221).

The five contract fixtures in §11 pin down exactly these three behaviors (plus round-trip and overwrite), so every platform implementation is checked against the same cases MSC 1's own comments already promised, not against a fresh guess per platform.

## 9. Key-naming scheme

MSC 1's `(service, account)` pair collapses to one dot-delimited string key: `<domain>.<secret>` for a single global secret, `<domain>.<secret>.<scope-id>` when MSC 1 scoped the secret per-server via its `account` field. Table below is the literal migration of all five of `KeychainManager`'s secrets — this is what wiring `SecretStore` into real pairing (the homeless gap Phase 3's scope doc already flags) will use, not a hypothetical extension:

| MSC 1 (`service`, `account`) | `SecretStore` key |
|---|---|
| `remoteapitoken`, `owner` | `remote-api.owner-token` |
| `remoteapiguesttoken`, `guest` | `remote-api.guest-token` |
| `xboxbroadcast.altpassword`, `<server UUID>` | `xbox-broadcast.alt-password.<server-id>` |
| `playit.secretkey`, `agent` | `playit.secret-key` |
| `curseforge.apikey`, `apikey` | `curseforge.api-key` |

No Rust constants for these five are defined yet — nothing in the codebase calls `SecretStore::get`/`set` until the pairing-flow wiring gap (`phase3-scope.md`) is picked up, so a constants module would have no caller and would just be guessed-at scaffolding. This table is the scheme that call site follows when it lands.

## 10. macOS Keychain scope, for P3.9

**Confirmed by Cameron Temple, 2026-08-01 (`service-identity.md` §3): the macOS `SecretStore` implementation targets the System keychain**, not the login keychain — restated here in full because this is the document P3.9 needs to be self-sufficient from, per this step's own charge to record the answer before P3.9 can start.

Why: a `LaunchDaemon` runs outside any `loginwindow` security session (`service-identity.md` §4). Setting `UserName` changes the Unix UID/GID the process runs under; it does not attach the process to a login session or to that session's keychain-unlock state. Whether a `UserName`-scoped `LaunchDaemon` can reach the login keychain in practice is genuinely untestable until Phase 4 builds a real `LaunchDaemon` — so P3.9 is unblocked by targeting the System keychain (`SecKeychain` system domain, not tied to any login session, reachable by any locally-running process including daemons) now, rather than waiting on a test that can't happen yet. If Phase 4's live test later shows login-keychain access does work, that's a strictly better outcome adoptable then, without having blocked this phase on it.

What this does and does not protect at rest, per §8's own requirement, restated in this platform's terms: per-item access-control lists restrict readability to the agent's own process, but the System keychain is a **machine-scoped** secret store, not a user-scoped one — recoverable by anything running as root on the same machine. Same category as this document's own Linux answer (§2); Windows is not in this category — Credential Manager wraps DPAPI's *per-user* mode, tied to the installing account rather than the whole machine (§13's cross-platform table has the accurate comparison across all three).

## 11. Contract fixtures

Five fixtures, `fixtures/secret-store-contract/*.json`, characterized directly from `KeychainManager`'s own primitives and their doc comments (§8 above), not pulled from a dedicated MSC 1 test file — none exists, the same "characterize from source" pattern P3.5 used for path safety. Every platform implementation (P3.9 macOS, P3.10 Windows, P3.11 Linux) runs these same five against itself; `crates/msc-infrastructure/src/secret_store.rs` also ships a `FakeSecretStore` (in-memory) that satisfies them today, so the contract is checkable before any platform crate exists.

| Case | What it pins down |
|---|---|
| `round-trip-set-then-get` | `set` then `get` returns the same value |
| `get-of-unset-key-returns-none` | Reading a never-set key returns `Ok(None)`, not an error |
| `set-overwrites-existing-key` | `set` on an existing key overwrites it (the upsert behavior `write`'s own comment names) |
| `delete-then-get-returns-none` | `delete` then `get` returns `Ok(None)` |
| `delete-of-unset-key-is-noop` | Deleting a never-set key is `Ok(())`, not an error (the `errSecItemNotFound`-is-acceptable behavior `delete`'s own comment names) |

---

## 12. P3.11 finding: `systemd-creds` doesn't fit `SecretStore`'s live API, and what to build instead

**Status: Confirmed** by Cameron Temple, 2026-08-01.

Sections 1–7 above picked `systemd-creds` as the Linux backend and confirmed it in writing. Building P3.11 against that choice surfaced something those sections did not anticipate: `systemd-creds` doesn't just have an open provisioning question (§4's "who runs `systemd-creds encrypt`, and when?") — it doesn't fit `SecretStore`'s `get`/`set`/`delete` shape *at all*, on any machine without a TPM chip. That includes plain cloud/VM Linux hosts and this project's own CI runners, so it's not an edge case.

**What was found, and how it was checked — not assumed:**

Both `systemd-creds encrypt` and `systemd-creds decrypt`, called directly by a running process outside of `systemd` itself starting a unit, require root. The host-key backend (§2's "host-key fallback," what any machine without a TPM2 chip uses) reads and writes `/var/lib/systemd/credentials.secret`, which is root-owned, mode `0600` — not just for provisioning (§4's already-flagged question), but for *every* encrypt or decrypt call, including reads. There is no unprivileged mode short of `--with-key=null`, which stores the value with no encryption at all, defeating the entire purpose of choosing this backend.

Confirmed against two independent sources, not memory: the `systemd-creds` manpage (via `man7.org/linux/man-pages/man1/systemd-creds.1.html`), and several still-open systemd upstream bug reports of other people hitting exactly this in practice — `systemd/systemd#30191` ("Allow per-user services (`--user`) to get systemd-creds encrypted credentials"), `#33318` ("Use SetCredentialEncrypted in user service"), `#36895` ("Podman user service SetCredentialEncrypted failed permission denied").

**Why this is a shape mismatch, not just a permissions inconvenience:** `systemd-creds`'s real design is "`systemd` itself, running as root (PID 1), decrypts a fixed, unit-file-defined list of credentials once, at the moment that unit starts, and hands the plaintext to the service via a `CREDENTIALS_DIRECTORY`." That is fundamentally a static, unit-start-time-only mechanism — adding or changing a credential means editing the unit file and reloading/restarting the service. `SecretStore::get`/`set`/`delete` is a live, on-demand API a running, unprivileged agent calls whenever it needs to — the same shape Keychain (P3.9) and Credential Manager (P3.10) both actually support natively. `systemd-creds` does not support that shape for an unprivileged caller at all.

**The decision — two tracks, not a silent pick:**

1. **Real target design (not built in this phase): a small privileged helper.** The installer sets it up once, at the same elevated moment it already writes the `systemd` unit file (`service-identity.md` §4's install-time elevation window) — the agent talks to the helper locally (a Unix socket, restricted to the installing user's own UID) whenever it needs to read, write, or delete a secret, and only the helper ever touches `systemd-creds`. This preserves `systemd-creds`'s real protection (TPM2-sealed where available) and keeps the routine, unprivileged agent process exactly as unprivileged as P3.1 already decided it should be. **Not built now** because it needs its own service registration (a second unit, its own install/start/stop lifecycle) — the same boundary `phase3-scope.md` already draws around the *agent's* own registration ("this phase only decides identity and ownership; it doesn't install anything... that's Phase 4's gate"). Building a second privileged component's registration now would quietly cross that same boundary a second time.
2. **v1 stand-in, built now (P3.11): a plain file, encrypted with a key the agent's own installing-user account owns — not root.** One file per secret under `$XDG_DATA_HOME/msc2/secrets` (falling back to `$HOME/.local/share/msc2/secrets`), each ChaCha20-Poly1305-encrypted with a per-installation key generated on first use and stored alongside it (`<base>/key`, mode `0600`; the `secrets/` directory itself is mode `0700`). The agent reads and writes this with **no elevation at any point** — unlike `systemd-creds`, which needs root for every call on a non-TPM2 machine. Implementation: `crates/msc-platform-linux/src/secret_store.rs`.

**What this does and does not protect against at rest, per §8's own requirement, restated for this backend specifically (not the `systemd-creds` answer §2 already gave — that answer describes the *real* design in track 1, not what's actually running in v1):** anything running as the same OS user account the agent runs as (the installing user, per P3.1) can read `<base>/key` and decrypt every stored secret. This is **not a new category of exposure** — it's the same "recoverable by anything with this account's access" shape `service-identity.md` already accepts for the installing-user design as a whole, and unlike the `systemd-creds` TPM2 path, it is not bound to this specific machine's hardware (a copied `key` file plus its secrets directory would decrypt on any machine). Weaker than TPM2-sealing, stronger than the `--with-key=null`/no-encryption path this finding ruled out, and needs no root at any point — which the real helper-based design (track 1) still has to earn later without weakening this baseline.

**Revisit trigger:** once Phase 4 lands real service registration for the agent itself, build track 1 (the privileged helper) and retire this file-based stand-in, rather than letting it become the permanent answer by default.

## 12A. P4.3/P4.41-P4.43 decision: build and prove the helper for the Linux service gate

**Status:** P4.3 implementation decision, amended by P5.33 after P4.41/P4.42.

P4.3 closed the revisit trigger above by choosing the privileged
`systemd-creds` helper for installed Linux services. P4.41 then implemented the
callable helper server/client, UID-restricted socket protocol, and
`systemd-creds` get/set/delete behavior; P4.42 made production Linux service
authentication use the helper client. The file-based `LinuxSecretStore` remains
available for local development, tests, and temporary non-service runs, but it
is not the accepted backend for installed-service authentication.

P4.43 is still the evidence step that records credential persistence in real
service processes across macOS, Linux, and Windows. Do not treat this section as
claiming the full all-OS service-process proof until that step is complete.

Full helper design: `docs/msc2/lifecycle/linux-credential-helper.md`.

The important boundary is unchanged from P3.1 and P3.11: installing service
units and the helper socket/service uses the one install-time elevation window;
routine agent operation, token creation, pairing, and lifecycle control do not
ask for `sudo`. The agent runs as the installing user and talks to a root-run
helper over a Unix socket restricted to that installing user's UID. Only the
helper touches `systemd-creds` and the root-owned encrypted credential blob
directory.

---

## 13. Cross-platform conformance summary (P3.12)

All three platform implementations' `secret_store_contract` suites ran in the same CI run, not as three isolated green checkmarks nobody compared: [run 30689870770](https://github.com/ctemple9/msc2/actions/runs/30689870770), `Toolchain (macos-latest)` / `(ubuntu-latest)` / `(windows-latest)` jobs, each green, each running its own five `secret_store::tests::secret_store_contract_*` (Linux additionally runs `key_file_and_secrets_dir_are_owner_only`, checking the `0600`/`0700` file modes §12's implementation promises).

| Platform | Backend | Scope | What it protects against at rest | What it does *not* protect against |
|---|---|---|---|---|
| **macOS** (P3.9, P4.42) | Install-time System-keychain root secret protecting the mutable encrypted agent-owned store. Direct routine System-keychain mutation by the LaunchDaemon was rejected after the live write probe failed. | Machine-scoped root material plus agent-owned durable data — not tied to any login session (§10) | Other ordinary users cannot read the root secret or the agent-owned encrypted store | Anything running as **root** on the same machine can recover or operate on the root material/store |
| **Windows** (P3.10) | Credential Manager (`CredWriteW`/`CredReadW`/`CredDeleteW`, `CRED_TYPE_GENERIC`), which wraps DPAPI for the actual at-rest encryption | User-scoped — DPAPI's per-user mode, tied to the installing user's own account (`CRED_PERSIST_LOCAL_MACHINE` only controls *persistence across logons*, not *who* can decrypt) | Anything without that Windows user account's own credentials/logon session cannot decrypt | Anything running **as that same Windows user account** (any process running as them can read their own Credential Manager store) |
| **Linux** (P3.11, P4.41/P4.42) | Installed services use the privileged `systemd-creds` helper client. The file-per-secret encrypted store from §12 remains a development/test and temporary non-service backend, not the installed-service answer. | Service credentials are mediated by the installing-user service plus a root-run helper socket restricted to that UID; encrypted blobs are root-owned and handled through `systemd-creds`. | Other unrelated OS accounts cannot read the helper socket or root-owned encrypted credential material; TPM2-backed hosts additionally bind the encrypted material to that machine. | Root on the same live machine can recover or operate on the credentials; non-TPM host-key fallback is only as strong as the root-owned host key. Full real-service persistence evidence remains P4.43. |

**The shape of the production answer is the same across all three, deliberately — not three unrelated designs that happen to coexist:** every installed-service backend is a durable platform secret path, never `FakeSecretStore`, and never a login-session-scoped store that a headless service cannot reliably access. The older Linux file store remains weaker against copied-disk/key exfiltration and is retained only for development, tests, and temporary non-service runs. P4.43 remains the step that records the final real-service persistence evidence across all three platforms.
