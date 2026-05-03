# Media and reviewer notes

![Qorx banner](assets/qorx-img.jpg)

This page is for editors, reviewers, and maintainers who need a short factual
summary of Qorx.

## Summary

Qorx Community Edition is an AGPL-licensed Rust source project for local context
resolution. It defines a small `.qorx` source format, compiled `.qorxb`
bytecode, and `qorx://` handles resolved by a local runtime.

Qorx is not a hosted AI service. It is not a general compression system. It
works when a workflow carries Qorx source, bytecode, evidence packs, or handles
and has a resolver available.

## Basic facts

| Field | Value |
| --- | --- |
| Project | Qorx Community Edition |
| Creator | Marvin Sarreal Villanueva |
| Location | Metro Manila |
| Current public version | 1.0.4 |
| Initial public release line | May 1, 2026 |
| Implementation language | Rust |
| Source extension | `.qorx` |
| Bytecode extension | `.qorxb` |
| Handle scheme | `qorx://` |
| License | AGPL-3.0-only |
| Repository | https://github.com/bbrainfuckk/qorx |
| Handbook | https://bbrainfuckk.github.io/qorx/ |
| Banner image | https://raw.githubusercontent.com/bbrainfuckk/qorx/main/docs/assets/qorx-img.jpg |

## Install surface

- Source build: https://github.com/bbrainfuckk/qorx
- Independent review brief: https://bbrainfuckk.github.io/qorx/INDEPENDENT_REVIEW.html

Official installers, package-manager wrappers, tray UX, provider routing, and
account activation belong to Qorx Local Pro, not Community Edition.

## Technical review questions

These are fair questions to test against the implementation:

- Is `.qorx` better described as a small language or as a configuration format?
- Is the `.qorxb` bytecode layer useful outside the CLI?
- Does resolving local handles reduce repeated prompt payloads in real workflows?
- What data should be allowed into local evidence packs?
- What are the operational boundaries for resolver trust, receipts, and cache?

## Boundaries

Qorx can resolve Qorx-known local handles, bytecode, indexed evidence, cache
entries, and receipts. It cannot reconstruct arbitrary unknown files from a tiny
message. It cannot make a remote model know hidden local data without a resolver
path. Its token accounting is deterministic local estimation unless another
tokenizer is explicitly named.

## Contact

Marvin Sarreal Villanueva

- marvin@orin.work
- msarvillan@gmail.com
