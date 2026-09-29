# Archived automation

`ci.yml` and `release.yml` are the workflows used before D-039. GitHub Actions
does not load workflows from this directory. The old release validators and CI
waiter are preserved under `tools/release/archive/`.

Test source remains in its original locations so it can be run manually if
Cameron requests it. The active release workflow only builds, assembles, signs,
and publishes artifacts.
