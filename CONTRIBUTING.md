# Contributing

Thanks for considering a contribution. For a large change, open an issue first
to agree on its scope. For a focused change, open a pull request with a short
description of the behavior and the verification you performed.

## Development setup

- Install the stable Rust toolchain with `rustfmt` and `clippy`, as selected by
  [`rust-toolchain.toml`](rust-toolchain.toml).
- Install Node.js 22 and npm.
- From `clients/desktop-web`, run `npm ci` to install the locked frontend
  dependencies.

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
that server. The CI workflow describes the automated checks used for pull
requests.

## Licensing contributions

By submitting a contribution for inclusion, you agree it may be distributed
under Apache-2.0, the same license as MSC 2's original code. You retain
copyright in your contribution and confirm that you have the right to submit
it. Third-party code must retain its own license and notices; do not copy it
into MSC 2 without documenting its source and terms.
