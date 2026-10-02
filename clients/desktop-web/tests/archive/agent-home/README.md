# Agent-home source assertions

Archived during P12.194 (2026-10-02). These files preserve the previous tests;
`.disabled` keeps them outside Vitest's test-file selection.

The retired assertions matched exact agent-page wording, button layouts and
implementation strings. They rejected the owner-approved redesign without
checking service behavior. The active agent-install file retains its two
controlled behavioral checks: an explicitly stopped service is not started
automatically, and a missing service is not installed automatically. The
first-launch reset file retains its remote identity and agent-owned setup
checks. No new tests or release gates were added.
