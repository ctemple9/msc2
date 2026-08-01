# Linux headless secret-storage backend (D-012 / `msc2-engineering.md` §8)

**Status: Proposed**, pending Cameron Temple's confirmation — 2026-08-01. Same pattern as every other judgment call in this register: this step reasons through the choice `msc2-engineering.md` §8 already leaned toward, states the answer in writing, and stops short of marking it Approved.

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

**Either way, this is a machine-scoped secret, not a user-scoped one** — the same category as the DPAPI machine-scope answer D-025 already gave for Windows, and the same category as the System-keychain default P3.1 just confirmed for macOS. Consequence, stated the same way those two were: anything running as root on the same machine can recover the secret (via the TPM, if present, or via the host key file). This is not a weaker answer than the other two platforms; it is the same shape of answer, for the same underlying reason — none of the three platforms give an unprivileged headless service access to a *session*-scoped secret store, because none of the three have a login session to scope to (D-025's own framing).

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

**This step cannot responsibly decide MSC 2's minimum supported Debian release** — that's a product/support-matrix decision, not a secret-storage implementation detail, and nothing in the audit corpus or `msc2-decisions.md` currently pins one. Flagged here because it is a direct, concrete consequence of this choice: if bullseye-or-older needs to be supported, `systemd-creds` cannot be the sole backend on Linux, and the root-owned-file fallback from §8's own table would need to be built as a real runtime fallback rather than left unbuilt. Recommendation: pin the minimum to Debian 12 (bookworm) or any distribution with `systemd` ≥ 250, since bookworm has been current stable since mid-2023 and is what a new minimal-Debian install performed today would produce — but this is Cameron's call, not one this step makes unilaterally.

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
| What does it *not* protect against? | Anything running as root on the same live machine — a machine-scoped secret, same category as Windows DPAPI and the macOS System-keychain default (D-025) |
| Does it require `DynamicUser=`? | No — works identically with P3.1's static installing-user `User=`/`Group=` |
| Who runs `systemd-creds encrypt`, and when? | The installer, during the same elevated install-time window that already writes the unit file (P3.1) |
| Minimum `systemd` version? | 250+, for the *encrypted* credential directives. Debian 12 (bookworm) ships 252 and qualifies; Debian 11 (bullseye) ships 247 and does not — flagged for Cameron, not decided here |
| Is the Secret-Service branch built? | No — one code path only, per §8's own stated reason to avoid two threat models |
