# Rust build warning review — 2026-10-30

Scope: the Cargo workspace on the owner's macOS host, including the `msc`
binary used by the TUI. This is a compiler/call-site cleanup, not a full
security or product-behavior audit. No tests were executed, release workflow
changed, background agent installed, or TUI layout changed.

## Findings and changes

- The production agent emitted 24 compiler warnings and 11 Clippy warnings.
  The other production workspace packages emitted no warnings in this pass.
- Most compiler warnings traced to `BedrockStore::read`, the disconnected
  direct-tile implementation. The live artifact handler calls `read_shared`,
  converts a safe saved Bedrock snapshot into private Anvil input, and uses
  `RendererStore`. The phase archive records that replacement in the P18
  shared-renderer work. Removed the old reader, direct-tile/session state,
  atlas helpers, sample texture downloader and its constants, and no-op
  cleanup of state that could no longer be populated. Preserved the removed
  fragments, including their obsolete empty-catalog test, in
  `archive/bedrock-direct-tiles.rs.txt`. This does not retire Bedrock maps;
  snapshotting, conversion, cache locks, cancellation and shared rendering
  remain active.
- `valid_tile_path` and the exporter polling wrappers had no callers, even
  from the old reader. They are included in that archive rather than retained
  as dead production functions.
- The active converter initialized an exit-status variable only to overwrite
  it on every successful loop exit. The loop now returns its exit status
  directly; shutdown, progress and server-start handling remain in place.
- `fetch_visible` was exposed to the worlds module while returning a private
  error type. Its only callers are in the same module and its Bedrock child.
  Narrowed the method's visibility instead of exposing the error type.
- Removed the unused `auth::forbidden` wrapper and obsolete camera query
  fields. Unknown query parameters remain ignored by Serde; live renderer
  requests use dimension and artifact path.
- Production Clippy findings were nested conditions and unwieldy TUI function
  signatures/types. Collapsed the flagged conditions, named the refresh-task
  tuple, and passed the existing view state to the workspace drawing function.
  No broad warning suppression was added.
- Compiling all targets exposed two stale integration-test dependencies:
  auth middleware called a lifecycle response helper through a module absent
  in the isolated auth harness, and the CLI service test constructed removed
  network arguments. Auth now uses its identical local response helper;
  the CLI test constructs the current local-only arguments. Removed an
  unreferenced Purpur fake-transport builder from provisioning tests. No new
  tests or test cases were added.

## Verification boundary

Completed without warnings: `cargo build -p msc-agent`,
`cargo check --workspace --all-targets`, and
`cargo clippy --workspace --bins --lib -- -D warnings`. Formatting and
`git diff --check` also passed. Compilation of test
and example targets checks their types without executing them. Native Windows
and Linux target-specific branches, the separate Tauri desktop project,
vendored renderer toolchains, and runtime Bedrock rendering were not verified
by this macOS Cargo pass. Existing platform-specific code was not removed
merely because macOS does not compile it.
