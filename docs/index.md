# Qorx Community Edition

![Qorx banner](assets/qorx-img.jpg)

This is the public AGPL source line for Qorx.

Qorx is a small domain-specific language, compiler, and local context-resolution
CLI. Community Edition covers the language, bytecode, local indexing, and
evidence commands. It does not include the official signed local product, tray
UX, provider routing, hosted account features, or managed local-vault
experience.

Current public version: `1.0.4`.

## Start here

- [Community boundary](COMMUNITY.md)
- [Install from source](INSTALL.md)
- [Language and runtime](QORX.md)
- [Handbook](handbook/README.md)
- [Science notes](handbook/science.md)
- [Command reference](COMMANDS.md)
- [Server boundary](SERVER.md)
- [SAFE-R anti-hype gate](SAFE-R.md)
- [Technical credibility](TECHNICAL_CREDIBILITY.md)
- [Independent review brief](INDEPENDENT_REVIEW.md)
- [Qorx 1.0.4 for Rust reviewers](QORX_1_0_4_RUST.md)
- [Benchmarks](benchmarks/README.md)
- [Qorx papers](papers/README.md)

## Public surface

- source build with Cargo.
- `.qorx` and `.qorxb` language/runtime checks.
- local evidence commands.
- claim-boundary docs.
- tests and benchmark fixtures.

## Product boundary

Qorx Local Pro is separate. Official binaries, auto-update, tray, daemon
management, provider routing, one-click CLI integrations, cloud capsule sync,
and team policy are not shipped from this public repository.
