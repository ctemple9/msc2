# Unsigned beta evidence

**Release tag:** ___________________________  
**Date (UTC):** ____________________________  
**Operator:** ______________________________

The first beta is intentionally unsigned. This record makes the limitation
explicit so a checksum pass is not mistaken for publisher authentication.
Unsigned status is expected; an undocumented signing claim is a release-gate
defect.

## What the release does not claim

- [ ] macOS is not Developer ID signed or notarized. Any ad-hoc sidecar
      signature required by Virtualization.framework is not publisher
      identity.
- [ ] Windows installers and binaries are not Authenticode signed; the
      unknown-publisher/SmartScreen warning is expected.
- [ ] Linux packages are not distributed through a signed MSC package
      repository.
- [ ] The beta has no production auto-update path or signed coordinated-update
      manifest.
- [ ] `SHA256SUMS`/`sha256sums.txt` is recorded as an integrity comparison,
      not a signature.

## Evidence to retain

Record the exact source of each observation without including credentials:

| Platform | Evidence source | Observation | Reference |
|---|---|---|---|
| macOS | Candidate workflow / release notice | `--no-sign`; no Developer ID/notarization claim | __________________ |
| Windows | Physical installer launch and/or `Get-AuthenticodeSignature` | Expected unsigned/unknown-publisher state | __________________ |
| Linux | Release contract and package source | No signed repository claim; checksum only | __________________ |

For Windows, the optional local check is:

```powershell
Get-AuthenticodeSignature .\msc2-<release>-windows-x86_64.msi |
  Format-List Status,StatusMessage,SignerCertificate
```

**Recorded Windows result:**

______________________________________________________________________________

______________________________________________________________________________

For the CI source record, retain the successful tag workflow URL and note the
`--no-sign` build evidence. Do not claim that the workflow proved a physical
installer launch, a real service, or a trusted publisher.

**Workflow URL / run:** _____________________________________________________

**Final signing result:** Expected unavailable / Unexpected claim (gate open)
