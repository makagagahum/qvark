# Qorx runtime

The runtime has four jobs:

1. Build a local index.
2. Emit handles or evidence packs.
3. Resolve handles into proof pages.
4. Record receipts and accounting.

## Index

`qorx index .` builds local quarks. A quark stores bounded text, path, line
range, hash, estimated mass, sparse terms, and structural signals.

## qosm

`qosm` is the local state store. It contains the index, cache, receipts,
provenance, lattice state, snapshots, and share records. It stays local unless
the operator exports a file.

## Daemon boundary

The daemon is not part of the Community Edition product surface. The public CE
binary refuses daemon commands:

```sh
qorx daemon start
qorx daemon status
qorx daemon stop
```

The official daemon, tray, provider routing, startup integration, and managed
local vault UX belong to Qorx Local Pro.

## Handles

| Handle | Meaning |
| --- | --- |
| `qorx://s/...` | Session state. |
| `qorx://c/...` | Capsule state. |
| `qorx://u/...` | Event receipt. |
| `qorx://l/...` | Lattice state. |
| `qorx://f/...` | File-share state. |

A handle is valid only if a resolver can verify and expand it.

## qshf

`qshf` is the ratio between local indexed mass and visible carrier mass. It is
useful for reasoning about context pressure. It is local accounting, not a
provider bill.

## B2C Quant Allocator

`qorx b2c-plan` scores indexed quarks and chooses a local evidence portfolio
under a token budget. `qorx pack` uses the same allocator before rendering the
visible context. The allocator is deterministic and local. It does not call a
provider.

## Failure Rule

If Qorx cannot resolve evidence, it should refuse or return the boundary. It
should not invent missing context.
