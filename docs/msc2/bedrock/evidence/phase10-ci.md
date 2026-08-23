# Phase 10 CI candidate

The exact Phase 10 candidate is commit
`2ccb1d0d509dcedb50e3f9c153845ee44934ff93`. GitHub Actions run
[`32655288252`](https://github.com/ctemple9/msc2/actions/runs/32655288252)
completed successfully on 2026-08-23 and exercised that commit directly.
The later P10.27 documentation commit is not the tested candidate.

## Green jobs

| Evidence | GitHub job | Result |
|---|---:|---|
| macOS toolchain and Phase 10 synthetic, compatibility, and evidence checks | `97232910629` | success |
| Linux toolchain (`ubuntu-latest`) and Phase 10 synthetic, compatibility, and evidence checks | `97232910758` | success |
| Windows toolchain and Phase 10 synthetic, compatibility, and evidence checks | `97232910707` | success |
| Native headless artifacts link no GUI frameworks | `97234439713` | success |

Each platform job also completed its build, formatting, Clippy, full workspace
test, and headless-artifact collection steps before the final headless link
check consumed the native artifacts.

## Limits

This run proves the offline tri-platform CI contract for the recorded commit.
The Phase 10 jobs used the synthetic fixtures and documentary checkers; they
did not download BDS, start a live Bedrock server or VM, require a Mojang
account, or make a public-network reachability claim. Live native and sidecar
claims remain bounded by the separate P10.24/P10.25 evidence records.
