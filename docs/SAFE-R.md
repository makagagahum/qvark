# SAFE-R Gate

SAFE-R means Substantiated, Auditable, Falsifiable, Evidence-bound,
Restricted-claims.

Use it before publishing docs, package notes, demos, or release copy. It is the
anti-hype gate for Qorx.

## Physics Vocabulary

Qorx keeps the physics words because the language is built around quarks. The
words are product vocabulary over local runtime objects.

| Qorx term | Runtime meaning |
| --- | --- |
| quark | Bounded, hashed, token-estimated evidence chunk. |
| Cosmos | Local protobuf state: index, cache, receipts, provenance, lattice, traces. |
| Redshift | Baseline-to-Compact ratio between local context mass and visible carrier mass. |
| photon | Model-visible carrier: prompt block, A2A message, or `qorx://` handle. |
| wavefunction | `.qorx` source before parsing. |
| collapse | Parsed opcodes or `.qorxb` bytecode. |
| event horizon | Boundary between local evidence and provider-visible tokens. |
| Quetta / `Q` | One-character resolver alias for the active local index manifest. |
| subatomic | Small working set under the requested token budget. |

These are not physics claims. Qorx is not a physics engine. It does not claim
physical compression, hidden data transfer, provider billing bypass, or invoice
savings without matching routed-provider evidence.

## Pass Rules

- A public claim must name the command or code path that proves it.
- A number must say whether it is a local estimate, a benchmark result, or a
  provider billing record.
- Redshift/B2C numbers are local accounting until provider usage or invoices
  prove billable savings.
- Research citations may explain why a design direction is reasonable. They do
  not prove Qorx performance by themselves.
- Adapter science is inactive until the adapter is configured and ready.
- Public SaaS claims require the external SaaS layer: auth, tenancy, backups,
  monitoring, rate limits, and published load data.

## Run

```powershell
.\scripts\safer-check.ps1 -Exe .\target\release\qorx.exe
```

For a fast pass after running Cargo checks yourself:

```powershell
.\scripts\safer-check.ps1 -Exe .\target\release\qorx.exe -SkipCargo
```

The script checks formatting, tests, clippy, package dry runs, local science
boundaries, adapter readiness, the billion-dollar claim guard, provenance
attestation, obvious secret patterns, and unsafe public wording.
