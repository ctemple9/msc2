Deliberately empty. `real-corpus-check.py --selftest`'s "empty" case: an
inventory directory with zero config files must fail with "need at least
2", before it ever gets to checking a manifest or a transfer package.
