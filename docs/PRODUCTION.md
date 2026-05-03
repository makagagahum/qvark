# Community status

This page defines the public Community Edition boundary.

## Verdict

Qorx Community Edition is suitable for source review, local experiments,
language/runtime testing, and research reproduction.

It is not the official production local product. It is not a hosted SaaS. It is
not a managed team runtime. It does not include signed installers, automatic
updates, tray UX, provider routing, account activation, or fleet controls.

## Ready in CE

| Surface | Status | Evidence |
| --- | --- | --- |
| `.qorx` source language | Ready | `qorx qorx <file>` and `qorx qorx-compile <file>` |
| `.qorxb` bytecode | Ready | `qorx qorx-inspect <file>` |
| Source build | Ready | `cargo test`, `cargo build --release` |
| Local indexing | Ready | `qorx index`, `qorx search` |
| Evidence commands | Ready | `qorx strict-answer`, `qorx pack`, `qorx squeeze`, `qorx judge` |
| Provenance checks | Ready | `qorx security attest`, `qorx security verify` |
| Operator check | Ready | `qorx doctor --json` |

## Not included in CE

| Surface | Boundary |
| --- | --- |
| Official binaries | Local Pro or maintainer-controlled channels only |
| Windows tray | Local Pro |
| Auto-update | Local Pro |
| Daemon startup | Local Pro |
| Provider proxy routing | Local Pro |
| One-click CLI integrations | Local Pro |
| Hosted account features | Qorx API |
| Cloud capsule sync | Qorx API or Local Pro |
| Team policy and fleet controls | Team/Enterprise product |
| Public SaaS runtime | Separate hosted product with auth, tenancy, logs, backups, and SLOs |

## CE gate

Run this before publishing Community Edition claims:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
qorx --version
qorx doctor --json
qorx index .
qorx security attest
```

Do not use the CE repo to advertise official production distribution channels.
Use Qorx Local Pro for the paid local runtime and Qorx API for hosted account
features.

## Allowed claim

Use this wording:

```text
Qorx Community Edition is the AGPL source line for the Qorx language, bytecode,
local indexing, and evidence-command model. The official local product is Qorx
Local Pro.
```

Do not present Community Edition as the complete official local product.
