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

P10.3's LevelDB and NBT storage fixtures likewise describe byte layouts and
parser outcomes derived from the Swift readers. They do not claim provenance
from a real BDS database. Corrupt inputs are labeled explicitly, and the world
layout set records the one deliberate Phase 3 path-safety correction separately
from MSC 1's observable folder-resolution behavior.
