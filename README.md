# Qorx Community Edition

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.19875352.svg)](https://doi.org/10.5281/zenodo.19875352)
[![Preprint DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.19953308.svg)](https://doi.org/10.5281/zenodo.19953308)
[![Software Heritage](https://img.shields.io/badge/Software%20Heritage-archived-ff6600)](https://archive.softwareheritage.org/browse/origin/directory/?origin_url=https://github.com/bbrainfuckk/qorx)
[![License: AGPL-3.0-only](https://img.shields.io/github/license/bbrainfuckk/qorx?color=blue)](LICENSE)
[![Rust stable](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)

![Qorx banner](docs/assets/qorx-img.jpg)

This repository is the AGPL Community Edition of Qorx.

Qorx is a small domain-specific language, compiler, and local context-resolution
CLI. It lets a workflow carry a checked `.qorx` program, `.qorxb` bytecode,
Qorx handle, or evidence pack instead of repeatedly pasting the same local files
into prompts.

Community Edition is useful for source review, local builds, research, tests,
and interoperability work. It is not the full official local product.

Qorx Local Pro is separate. Signed installers, tray UX, auto-update, background
runtime management, provider routing, one-click CLI integrations, hosted account
features, cloud capsule sync, team policy, and managed local-vault UX are not
part of this public repository.

## Status

Current public version: `1.0.4`.

Qorx is free software under `AGPL-3.0-only`. The Qorx name, logo, product marks,
and official distribution identity are separate from the code license. Forks are
allowed under the license, but they may not imply that they are official Qorx.
See [TRADEMARKS.md](TRADEMARKS.md).

## Measured example

On the Qorx repository itself, the local benchmark in
[`docs/benchmarks/2026-05-02-qorx-self.md`](docs/benchmarks/2026-05-02-qorx-self.md)
reports:

| Case | Indexed local tokens | Model-visible tokens | Local reduction |
| --- | ---: | ---: | ---: |
| Session carrier | 219,838 | 73 | 3,011.48x |
| Evidence pack | 219,838 | 484 | 454.21x |
| Squeeze extract | 219,838 | 419 | 524.67x |

These are Qorx local `ceil(chars / 4)` estimates. They are not provider invoice
savings, and they do not prove answer quality. They show the boundary Qorx is
built to measure: large local state, small visible carrier, resolver available.

## Read first

- [Community boundary](docs/COMMUNITY.md)
- [Install from source](docs/INSTALL.md)
- [Language](docs/handbook/language.md)
- [Runtime notes](docs/handbook/runtime.md)
- [Command reference](docs/COMMANDS.md)
- [Server boundary](docs/SERVER.md)
- [SAFE-R anti-hype gate](docs/SAFE-R.md)
- [Technical credibility](docs/TECHNICAL_CREDIBILITY.md)
- [Independent review brief](docs/INDEPENDENT_REVIEW.md)
- [Qorx 1.0.4 for Rust reviewers](docs/QORX_1_0_4_RUST.md)
- [Benchmarks](docs/benchmarks/README.md)
- [Papers and preprint](docs/papers/README.md)

Qorx is not a prompt trick, a billing bypass, a general-purpose language, or
universal compression of unknown data. It works when a workflow carries Qorx
source, bytecode, handles, or evidence packs and has a resolver available.

## Minimal program

```text
QORX 1
use std.evidence
use std.branch as br
let question = "which files explain how Qorx keeps local evidence outside the model prompt?"
let fallback = "qv0d: local evidence does not support this answer"
pack evidence from question budget 700
cache evidence key question ttl 3600
strict answer from evidence limit 2
if supported(answer) then emit answer else emit fallback
```

Check it:

```powershell
cargo run -- qorx-check .\goal.qorx
```

Run it from source:

```powershell
cargo run -- qorx .\goal.qorx
```

Compile it:

```powershell
cargo run -- qorx-compile .\goal.qorx --out .\goal.qorxb
cargo run -- qorx .\goal.qorxb
```

## Core model

| Term | Short name | Meaning |
| --- | --- | --- |
| `.qorx` | qwav | Human-readable source. |
| `.qorxb` | qfal | Protobuf-envelope bytecode after semantic checks and compile. |
| QIR | qir | Lowered Qorx intermediate representation used for compiler-visible resolver calls. |
| stack tape | qstk | Forth-inspired bytecode word stream for tiny local dispatch. |
| cache policy | qcas | Source-level cache binding for stable resolver outputs near the runtime. |
| resolver step | qop | Named opcode such as `pack`, `strict`, `squeeze`, `map`, or `session`. |
| carrier | phot | Small model-visible object: source, bytecode, handle, or evidence pack. |
| `qorx://s/...` | qses | Session handle for indexed local state. |
| `qorx://c/...` | qcap | Capsule handle for a local context bundle. |
| `qorx://u/...` | qevt | Event handle for a local receipt. |
| quark | qrk | Bounded, hashed, token-estimated evidence chunk. |
| local state | qosm | Local Qorx state: index, cache, receipts, provenance, lattice, traces. |
| resolver boundary | hzon | Line between local state and model-visible carrier. |
| qshf factor | qshf | Baseline-to-Compact ratio between local context mass and visible carrier mass. |
| B2C | b2c | Baseline-to-Compact accounting. Local estimate, not a provider invoice. |
| B2C allocator | qalc | Local budgeted quark selector used by `b2c-plan` and `pack`. |

These are Qorx vocabulary labels, not physics claims. The full boundary is in
[SAFE-R](docs/SAFE-R.md).

## Build

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Install from public source

Use Cargo against the current public source branch:

```sh
cargo install --git https://github.com/bbrainfuckk/qorx --branch main --locked qorx
```

Or clone the repository and build:

```sh
git clone https://github.com/bbrainfuckk/qorx.git
cd qorx
cargo test
cargo build --release
```

## Community commands

```powershell
.\target\release\qorx.exe --version
.\target\release\qorx.exe doctor --json
.\target\release\qorx.exe index .
.\target\release\qorx.exe strict-answer "what proves Qorx is a language runtime?"
.\target\release\qorx.exe b2c-plan "what proves Qorx is a language runtime?" --budget-tokens 900
.\target\release\qorx.exe pack "what proves Qorx is a language runtime?" --budget-tokens 1200
.\target\release\qorx.exe security attest
```

The public CE binary refuses Pro-only commands such as `bootstrap`, `daemon`,
`tray`, `startup`, `drive`, `hot`, `integrate`, `run`, and `patch`.

## Repository map

| Path | Purpose |
| --- | --- |
| `src/` | Rust implementation of the parser, runtime components, index, protocol, and CLI. |
| `tests/` | Runtime, language, capsule, context, lattice, and strict evidence tests. |
| `docs/handbook/` | Manual-style operating documentation. |
| `docs/COMMANDS.md` | Community command catalog. |
| `docs/COMMUNITY.md` | Public/private product boundary. |
| `docs/SERVER.md` | Server and daemon boundary for CE. |
| `examples/` | Small fixtures for impact and evidence routes. |
| `scripts/` | Proof, benchmark, and maintainer checks. |

## Boundaries

Qorx can resolve Qorx-known local handles, bytecode, indexed evidence, and
receipts. It cannot reconstruct arbitrary unknown files from a tiny message. It
cannot make a remote model know hidden local data without a resolver path. It
does not certify task quality by token savings alone.

The official commercial experience is Qorx Local Pro. Do not describe forks,
community builds, or self-built binaries as official Qorx products.

## License

Copyright (c) 2026 Marvin Sarreal Villanueva.

- Code and operational docs: [AGPL-3.0-only](LICENSE)
- Citation metadata: [CITATION.cff](CITATION.cff)
- Qorx Local Context Resolution preprint: [10.5281/zenodo.19953308](https://doi.org/10.5281/zenodo.19953308)
- Contribution terms: [CONTRIBUTING.md](CONTRIBUTING.md)
- Security policy: [SECURITY.md](SECURITY.md)
- Governance: [GOVERNANCE.md](GOVERNANCE.md)
- Marks and official identity: [TRADEMARKS.md](TRADEMARKS.md)
