# Qorx Community command reference

This reference assumes the binary is built from source.

```powershell
cargo build --release
```

Use `.\target\release\qorx.exe help` for the live command tree.

## Proof gate

Run this before publishing Community Edition claims:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
.\target\release\qorx.exe --version
.\target\release\qorx.exe doctor --json
.\target\release\qorx.exe index .
.\target\release\qorx.exe b2c-plan "language runtime proof" --budget-tokens 900
.\target\release\qorx.exe strict-answer "language runtime proof"
.\target\release\qorx.exe security attest
.\scripts\safer-check.ps1 -Exe .\target\release\qorx.exe
.\scripts\check-testsprite-enterprise.ps1
```

## Language

```powershell
.\target\release\qorx.exe qorx .\goal.qorx
.\target\release\qorx.exe qorx-check .\goal.qorx
.\target\release\qorx.exe qorx-compile .\goal.qorx --out .\goal.qorxb
.\target\release\qorx.exe qorx-inspect .\goal.qorxb
.\target\release\qorx.exe qorx-prompt .\goal.qorx
.\target\release\qorx.exe lexicon
```

Minimal source:

```text
QORX 1
let question = "which files explain how Qorx keeps local evidence outside the model prompt?"
pack evidence from question budget 700
cache evidence key question ttl 3600
strict answer from evidence limit 2
assert supported(answer)
emit answer
```

## Evidence

```powershell
.\target\release\qorx.exe index .
.\target\release\qorx.exe search "language runtime proof"
.\target\release\qorx.exe strict-answer "language runtime proof"
.\target\release\qorx.exe b2c-plan "language runtime proof" --budget-tokens 900
.\target\release\qorx.exe pack "language runtime proof" --budget-tokens 1200
.\target\release\qorx.exe squeeze "language runtime proof" --budget-tokens 700
.\target\release\qorx.exe judge "Qorx is a local context resolver."
```

## Reports

```powershell
.\target\release\qorx.exe doctor --json
.\target\release\qorx.exe stats
.\target\release\qorx.exe stats reset
.\target\release\qorx.exe adapters
.\target\release\qorx.exe science
.\target\release\qorx.exe security attest
.\target\release\qorx.exe security verify
.\target\release\qorx.exe bench
```

## Pro-only commands

The CE binary refuses these commands:

```text
bootstrap
daemon
tray
startup
drive
hot
integrate
run
patch
```

Those commands belong to Qorx Local Pro because they provide the official local
runtime experience: background gateway, tray, account activation, provider
routing, startup integration, and managed local vault behavior.

## Boundary

Community Edition is a source-build CLI. Do not describe a self-built CE binary
as the official Qorx local product.
