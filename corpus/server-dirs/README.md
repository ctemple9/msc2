`corpus/server-dirs/` stays intentionally empty in git.

Phase 4's live lifecycle gate needs a real imported Paper server directory, not a fabricated fixture tree. Point `MSC2_PHASE4_PAPER_SERVER` at one complete server directory on this machine, then run:

`python3 tools/phase4/live-paper-lifecycle-check.py --server-dir "$MSC2_PHASE4_PAPER_SERVER" --base-url http://127.0.0.1:48001`

Minimum contents for that directory:

- `paper.jar`
- `server.properties`
- any worlds, plugins, or config files needed for the server to boot normally on this host

The check owns its own temporary agent process and journal directory. It does not copy a server corpus into this repository.
