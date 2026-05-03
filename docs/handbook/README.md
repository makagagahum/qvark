# Qorx Handbook

Qorx is a small domain-specific language and runtime for local context
resolution.

Read this handbook before treating a `qorx://` handle as meaningful. A handle is
only an address. The resolver is the authority.

## Contents

- [Language](language.md)
- [Runtime](runtime.md)
- [Science Notes](science.md)
- [Protocol](protocol.md)
- [Operations](operations.md)
- [Claims](claims.md)
- [SAFE-R Gate](../SAFE-R.md)
- [Technical Credibility](../TECHNICAL_CREDIBILITY.md)
- [Install](../INSTALL.md)
- [Server And Daemon](../SERVER.md)
- [Production Status](../PRODUCTION.md)

## Rule

Qorx does not make hidden context available by naming it. It makes local context
addressable, auditable, and faultable when the resolver is present.

## Objects

| Object | Role |
| --- | --- |
| `.qorx` | Source program. |
| `.qorxb` | Compiled bytecode. |
| `qorx://s/...` | Session handle. |
| `qorx://c/...` | Capsule handle. |
| `qorx://u/...` | Event receipt handle. |
| qosm | Local state store. |
| qshf | Baseline-to-Compact local accounting. |
| quark | Bounded local evidence chunk. |
| B2C quant allocator | Local budgeted quark selector used by `b2c-plan` and `pack`. |

## Proof Discipline

Before publishing a claim, run the proof gate in
[`docs/COMMANDS.md`](../COMMANDS.md). If a claim depends on provider billing,
measure provider billing. If it depends on retrieval correctness, measure
retrieval correctness. qshf is local accounting, not an invoice.
