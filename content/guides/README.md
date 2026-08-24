# Educational content corpus

This directory is the agent-owned, reviewable source for structured guide data.
`content/help/` holds one Markdown topic per `id`; this directory holds the
ordered records that do not belong to a screen: the Concept Guide, the
first-launch tour, and router-guide data.

The router catalog is data only. Matching a router name, selecting a fallback,
substituting runtime values, and evaluating troubleshooting rules remain
executable behavior in `crates/msc-domain/src/router/`.

The source paths in these records are citations into the MSC 1 baseline at
`fccd61f0ed743086f1f5db6bef58e228a36010f3`. They describe the source of the
copy; they are not a runtime dependency.
