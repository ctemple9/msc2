# Bedrock reference corpus

No raw Bedrock server directory or LevelDB database was available in the
MSC 1 repository or on the owner's machine for this extraction step. The
fixture files in `fixtures/bedrock-*` are therefore source-derived behavior
cases, not claims about a captured live world. They use only the key shapes,
sentinel strings, and control-flow outcomes stated by the MSC 1 implementations.

A real Bedrock corpus can be added later when an owner-supplied artifact is
available. It must be sanitized and provenance-recorded before any fixture
claims it as `corpus_source`; this directory is intentionally not filled with
invented world or player data.
