# Dependency and release provenance

**Owner:** Cameron Temple

## Review policy

- Keep Rust dependencies in `Cargo.lock` and frontend dependencies in `clients/desktop-web/package-lock.json`. Release builds use locked installs (`cargo --locked` where applicable and `npm ci`). Review lockfile diffs as source changes, including transitive package additions and version changes.
- Review dependency advisories when GitHub or a package maintainer reports them and during each dependency-update change. Triage critical and high severity reports within seven days; assess lower severities in the next planned maintenance pass. The owner records the affected component, reachable use, severity, chosen fix or deferral, and due date in the change record.
- Review licenses when adding or upgrading a direct dependency and before each public release. Do not publish a dependency with an incompatible or unclear license until Cameron records an explicit exception here. An exception must name the package, version or version range, license or unresolved question, reason, scope, owner, and review date. There are no standing exceptions at the time this policy was added.
- Dependency inventories and advisory scans help review known metadata. They do not prove that a dependency is safe, that a release is free of vulnerabilities, or that published bytes came from trusted infrastructure.

## Immutable workflow references

The release workflow pins third-party GitHub Actions to full commit SHAs. Update a pin only through a reviewed change that records the upstream version beside the SHA. Rust and Node.js versions are pinned in workflow configuration. The former CI and provenance workflow is archived under D-039.

## Release records

The active beta release publishes nine platform assets, `SHA256SUMS`, and an Ed25519-signed update manifest. The manifest records release ID, tag, asset sizes, and digests; the signature is the authenticity check for installable assets. Dependency inventory, builder-environment, and extended provenance generation were removed from automatic publication by D-039. The former checker is preserved at `tools/release/archive/check-provenance.py` for historical reference; it expects the old CI evidence format and is not wired into the active workflow.
