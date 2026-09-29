# Testing and release workflow policy

**Owner:** Cameron Temple · **Recorded:** 2026-09-28

## Cameron's assessment

Cameron values tests when they protect important behavior. He is deeply
unhappy with how agents have used them on MSC 2. He estimates that the project
has accumulated more than 1,000 tests and does not believe they were all
essential. In his view, many are brittle or low-value, and the project has
spent more time waiting for workflows than developing code. Tests and checks
that fail for formatting, runner timing, fixture cleanup, or unrelated details have made
small changes expensive and releases unpredictable. This is a criticism of the
agents' decisions and test quality, not a rejection of testing itself.

The P16.29 beta release attempts made the cost unmistakable. The release waited
for a full same-commit CI matrix, while packaging failures only became visible
after another long run. Separate commits and reruns addressed Windows metadata,
Linux RPM inspection, and macOS smoke cleanup. Moving an unpublished release
tag between attempts further obscured which source was being released. A simple
beta publication consumed hours and still did not produce a release. Cameron's
time and attention were wasted.

## Rules for agents

1. **Add only essential tests.** Before adding one, name the user-visible
   failure or data/security risk it would catch, why current coverage does not
   catch it, and how the test will fail for that behavior alone. An agent may
   add an essential test without prior approval when that case is concrete;
   record the reason in the change. A desire for more coverage is insufficient.
2. **Make tests robust.** Prefer observable behavior over internal structure,
   exact prose, incidental formatting, timing, or snapshots. Use controlled
   inputs and cleanup. Avoid live networks, arbitrary sleeps, global state,
   machine-specific paths, and broad fixtures unless the real contract needs
   them. A flaky test is a defect to repair or retire, not a gate to rerun until
   it passes.
3. **Keep the suite small.** Reuse a focused check that already protects the
   behavior. Archive redundant and brittle checks when their value does not
   justify their upkeep; preserve their source. Record the reason for each new
   test and its expected runtime. Do not expand a broad matrix for one narrow
   behavior.
4. **Do not run tests by default.** An agent runs a test only when Cameron
   explicitly requests that specific command. A plan's `Verify:` line does not
   authorize the agent to run it. Cameron remains the verifier of step work.
5. **Keep release publishing independent of test suites.** The active beta
   workflow builds the supported artifacts, requires the complete asset set,
   generates checksums, signs update metadata, and publishes. Do not add CI,
   lint, smoke, or test jobs to that path without Cameron's explicit approval.
   Test results and physical acceptance can be recorded separately.
6. **Respect the cost of a release attempt.** Inspect all available failures
   before changing code. Check workflow syntax and packaging assumptions
   locally without running tests. Explain the expected fix and uncertainty.
   Do not move a release tag or launch another full release run as a routine
   debugging step. A failure in a build or publish step should lead to a
   targeted correction, with no unrelated gate added to compensate.

These rules govern future agent work. The archived workflows and existing test
source remain available for deliberate, owner-directed use. A successful build
is evidence that artifacts were produced; it is not evidence that every product
behavior has been verified. The Phase 16 exact-artifact acceptance record stays
open until Cameron records the required results.
