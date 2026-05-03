# Qorx Benchmark Report

Generated: `2026-05-01T15:49:01+00:00`

Suite: `benchmark-lab`

Target: `examples/benchmark-lab`

Qorx version: `qorx 1.0.2`

Git commit: `15caf6b`

## Summary

| Metric | Value |
| --- | ---: |
| Indexed local tokens | 1125 |
| Session visible tokens | 69 |
| Session reduction | 16.30x |
| Pack used tokens | 512 |
| Pack reduction | 2.20x |
| Squeeze used tokens | 391 |
| Squeeze reduction | 2.88x |
| Bench average reduction | 2.31x |
| Strict task pass rate | 100.0% |
| Expected refusal pass rate | 100.0% |
| Agent provider calls | 0 |

## Strict Tasks

| Question | Expected | Actual | Pass | Evidence | Used tokens |
| --- | --- | --- | ---: | ---: | ---: |
| local context resolution resolver boundary proof page | supported | supported | yes | 3 | 270 |
| galactic banana escrow treaty | not_found | not_found | yes | 0 | 8 |

## Bench Rows

| Query | Used tokens | Omitted tokens | Reduction | Quarks |
| --- | ---: | ---: | ---: | ---: |
| local context resolution resolver boundary proof page | 512 | 613 | 2.20x | 3 |
| qorx carriers .qorx .qorxb qorx handle | 508 | 617 | 2.21x | 3 |
| strict answer refusal unsupported claims | 446 | 679 | 2.52x | 3 |

## Boundary

This benchmark uses Qorx local accounting only. Token counts are deterministic
`ceil(chars / 4)` estimates unless the runtime reports another estimator. The
report does not claim provider invoice savings, production throughput, or
downstream model answer quality.

To reproduce:

```powershell
python scripts/run-benchmark.py --target examples/benchmark-lab
```
