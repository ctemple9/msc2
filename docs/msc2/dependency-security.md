# Dependency and release provenance

**Owner:** Cameron Temple

## Review policy

- Keep Rust dependencies in `Cargo.lock` and frontend dependencies in `clients/desktop-web/package-lock.json`. Release builds use locked installs (`cargo --locked` where applicable and `npm ci`). Review lockfile diffs as source changes, including transitive package additions and version changes.
- Review dependency advisories when GitHub or a package maintainer reports them and during each dependency-update change. Triage critical and high severity reports within seven days; assess lower severities in the next planned maintenance pass. The owner records the affected component, reachable use, severity, chosen fix or deferral, and due date in the change record.
- Review licenses when adding or upgrading a direct dependency and before each public release. Do not publish a dependency with an incompatible or unclear license until Cameron records an explicit exception here. An exception must name the package, version or version range, license or unresolved question, reason, scope, owner, and review date. There are no standing exceptions at the time this policy was added.
- Dependency inventories and advisory scans help review known metadata. They do not prove that a dependency is safe, that a release is free of vulnerabilities, or that published bytes came from trusted infrastructure.

## Immutable workflow references

Release and required CI workflows pin third-party GitHub Actions to full commit SHAs. Update a pin only through a reviewed change that records the upstream version beside the SHA. Rust, Node.js, and cargo-nextest versions are pinned in workflow configuration; the release provenance record captures the actual Rust, Cargo, Node.js, and npm versions used by its metadata job.

## Release records

Each release carries `DEPENDENCY-INVENTORY.json`, generated from the two lockfiles and the signed update manifest's component map, one `BUILD-ENVIRONMENT-<platform>.json` record per release platform, and `RELEASE-PROVENANCE.json`. The provenance record joins the source commit and tag to actual runner image versions and tool versions, manifest and lockfile digests, and staged artifact digests. `tools/release/check-provenance.py` checks that the records agree with the checked-out source, signed-manifest contents, and staged bytes. The update manifest signature remains the authenticity check for installable assets; provenance metadata and dependency scans are review evidence, not a substitute for signature verification or a security audit.
