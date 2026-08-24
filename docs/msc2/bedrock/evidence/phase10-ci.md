# Phase 10 CI candidate

The exact corrected post-P10.36 candidate is commit
`3226f7b4573fa0c3b058f134ccf79f0cd99388cb`. GitHub Actions run
[`32692896100`](https://github.com/ctemple9/msc2/actions/runs/32692896100)
completed successfully on 2026-08-24 and exercised that commit directly.
The later P10.37 documentation commit is not the tested candidate.

## Green jobs

| Evidence | GitHub job | Result |
|---|---:|---|
| macOS toolchain, full workspace regression, production-router smoke, P10.36 check, compatibility, and evidence checks | `97329642447` | success |
| Linux toolchain (`ubuntu-latest`), full workspace regression, production-router smoke, P10.36 check, compatibility, and evidence checks | `97329642474` | success |
| Windows toolchain, full workspace regression, production-router smoke, P10.36 check, compatibility, and evidence checks | `97329642355` | success |
| Native headless artifact link check, including the P10.36 check | `97331600727` | success |

Each platform job completed its build, formatting, Clippy, full workspace test,
the production-router smoke, the P10.36 production wiring check, compatibility
and evidence checks, and headless-artifact collection. The separate headless
job then consumed those native artifacts and completed its no-GUI link check.

## Limits

This run proves the offline tri-platform CI contract for the recorded commit.
The Phase 10 jobs used the synthetic fixtures and documentary checkers; they
did not download BDS, start a live Bedrock server or VM, require a Mojang
account, or make a public-network reachability claim. Live native and sidecar
claims remain bounded by the separate P10.24/P10.25 evidence records.

## P10.36 production wiring guard

P10.36 adds `tools/phase10/phase10-production-check.py --check` to every
Linux, Windows, and macOS toolchain leg and to the headless no-GUI link job.
The check is offline and source-level. It verifies that Bedrock capabilities
come from the production runtime selection, that the frozen public response
DTOs retain their additive runtime state, that no literal Bedrock refusal has
returned to production Rust, and that the cross-backend smoke launches the
real `msc serve` composition root. It does not download BDS, start a live
Bedrock server or VM, contact a provider, or make a public-network claim.

The exact post-P10.36 CI candidate and its results are recorded above by P10.37;
this note does not substitute this documentation change for that candidate.
