# Contributing

Thanks for considering a contribution. For a large change, open an issue first
to agree on its scope. For a focused change, open a pull request with a short
description of the behavior and the verification you performed.

## Where things live

- `crates/` contains the Rust agent, API, domain rules, and platform code.
- `clients/desktop-web/` contains the Svelte desktop interface and its Tauri
  host in `src-tauri/`.
- `content/help/` contains the handbook shown in the app.
- `docs/msc2/` contains the product vision, engineering contract, decisions,
  and current [rolling plan](docs/msc2/rolling-plan.md).
- `tools/` and `packaging/` contain development helpers and distribution files.

## Development setup

- Install the stable Rust toolchain with `rustfmt` and `clippy`, as selected by
  [`rust-toolchain.toml`](rust-toolchain.toml).
- Install Node.js 22 and npm.
- From `clients/desktop-web`, run `npm ci` to install the locked frontend
  dependencies.

From the repository root, start the host-local terminal interface with:

~~~sh
cargo run -p msc-agent -- tui
~~~

To launch the desktop app in development, run this from `clients/desktop-web`:

~~~sh
npx tauri dev
~~~

The Tauri development command builds the frontend and prepares a development
agent through the commands in `src-tauri/tauri.conf.json`. For service setup and
platform requirements, see the [engineering guide](docs/msc2/msc2-engineering.md).

Useful checks include:

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format
~~~

~~~sh
cd clients/desktop-web
npm run format:check
npm run check
npm run build
~~~

Choose checks that cover the files you changed. Do not claim a platform or
Minecraft lifecycle was verified unless it was run on that platform or against
that server. Pull requests do not currently run an active CI workflow; describe
the checks you ran in the pull request.

## Licensing contributions

By submitting a contribution for inclusion, you agree it may be distributed
under Apache-2.0, the same license as MSC 2's original code. You retain
copyright in your contribution and confirm that you have the right to submit
it. Third-party code must retain its own license and notices; do not copy it
into MSC 2 without documenting its source and terms.
