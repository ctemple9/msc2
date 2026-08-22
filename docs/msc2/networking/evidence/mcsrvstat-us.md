# mcsrvstat.us read-only probe

This evidence is a read-only `GET https://api.mcsrvstat.us/3/example.invalid` request
run on 2026-08-22. It returned HTTP success with `online: false` and a DNS lookup
failure for `example.invalid`. It creates no account, does not alter a server, and is
retained only to establish that the Phase 9 provider can be contacted independently of
a local Minecraft listener.
