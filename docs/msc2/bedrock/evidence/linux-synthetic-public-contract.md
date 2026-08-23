# Linux synthetic public-contract evidence

This is P10.13's reproducible, offline evidence record. It proves that the
shared Bedrock runtime lifecycle can cross the HTTP-shaped contract and that
the real `msc` CLI can drive the same lifecycle verbs against a disposable
loopback server. The boundary is an in-memory fake BDS runtime; this is not a
claim that a native Linux BDS distribution is supported. The application
service itself is exercised by the targeted `msc-application` test in the
same smoke command.

## Cell and boundary

- Host: Linux contract shape, exercised on the developer host through a
  loopback socket
- Architecture: test-build architecture
- Backend: synthetic native-style Bedrock runtime
- BDS distribution/version: none; the fake boundary identifies version
  `1.21.80.3` only as fixture data
- Network and credentials: no public network, account, private world, or
  unrestricted download

## Reproduction

Run from the repository root:

```text
bash tools/phase10/linux-smoke.sh --synthetic
```

The route proof covers provision, start, readiness/status, command, stop, and
process metrics. The CLI proof covers status, start, command, stop, and
capabilities. Both proofs also require the Bedrock capability response to say
`supported: false` with `backend: null` when no live runtime is available.

## Limits

This evidence is synthetic and does not replace P10.24's live Linux runtime
evidence. It must not be used to mark the Linux native Bedrock matrix cell
`supported`.
