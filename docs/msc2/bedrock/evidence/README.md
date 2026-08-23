# Bedrock compatibility evidence

This directory is the evidence boundary for the separate Bedrock runtime
matrix in `../compatibility-matrix.csv`. The matrix is not derived from the
MSC-agent or Java-server support claims. A host can support the agent and Java
while its Bedrock runtime is still unavailable.

## Matrix rules

Each row names one host/architecture/backend cell and carries two independent
claims:

- `agent_host_status` says whether the MSC agent host is supported.
- `bedrock_runtime_status` says whether that named Bedrock backend is supported.

The status vocabulary is:

- `supported` — reproducible evidence proves the claim;
- `unsupported` — reproducible testing proves the backend cannot run there;
- `unavailable` — the project cannot safely test or provide the backend, with
  an evidence-backed reason; and
- `planned` — not advertised yet, so it must have no support claim or evidence
  reference. Later evidence steps replace this value with a proved status.

Every advertised status (`supported`, `unsupported`, or `unavailable`) must
name a reproducible repository-relative evidence file in its corresponding
evidence column. The checker resolves an optional `#section` anchor against
the file before accepting the reference. An evidence file must describe the
exact host/backend cell, how it was tested, the result, and the command or
environment needed to reproduce it. A fixture or fake runtime can prove a
shared rule, but cannot by itself prove a supported native or sidecar cell.

The Apple Silicon row is mandatory under D-028. It remains a distinct row
from `macOS (Intel)`, and its Bedrock status is exactly `unavailable` with the
reason `no test hardware`. It must never be changed to `unsupported` merely
because the Intel-only appliance has not been run on Apple Silicon.

## Evidence record shape

Runtime evidence added by P10.24/P10.25 should be a committed JSON or Markdown
record under this directory. It must identify:

1. the matrix `host`, `architecture`, and `bedrock_backend` cell;
2. the exact BDS distribution/version or the explicit reason it was
   unavailable;
3. the host, OS version, and required libraries or virtualization resources;
4. the command or smoke-test entry point, its result, and capture time; and
5. any limits on the claim, including whether the result is synthetic or live.

P10.24's structured distribution records use the schema
`msc2.phase10.distribution-evidence.v1`. An `unavailable` package identity is
deliberate: it must name what the official source proves and why an exact
versioned archive and digest were not safely captured. It must not fill in a
fixture digest or a digest copied from the third-party MSC 1 reference
manifest. A `supported` matrix cell is accepted only when its matching record
has a verified official archive URL, captured archive, and SHA-256 identity.

Unavailable evidence is a real outcome, not a missing row. For D-028, the
decision record is the reproducible authority for the no-test-hardware reason;
future Apple Silicon evidence must replace that reference only after the
owner's stated revisit condition is met.

The checker intentionally rejects a missing evidence reference, an evidence
reference to a nonexistent file, a `planned` cell that pretends to be
supported, duplicate host/backend cells, and a missing or merged Apple
Silicon cell.
