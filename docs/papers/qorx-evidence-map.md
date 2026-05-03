# Qorx Evidence Map

| Claim | Proof path |
| --- | --- |
| Qorx has a language surface | `qorx qorx`, `qorx-compile`, `qorx-inspect`, language tests. |
| Qorx has bytecode | `.qorxb` compiler and runtime execution tests. |
| Qorx has local handles | `session`, `capsule`, `context nano`, `context quetta`, `context expand`. |
| Qorx resolves evidence | `strict-answer`, `pack`, `squeeze`, `context fault`. |
| Qorx records local state | `cosmos status`, `context snapshot`, `context verify`. |
| Qorx signs provenance | `security attest`, `security verify`. |
| Qorx reports local accounting | `session`, `bench`, `stats`. |

## Required Gate

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
.\target\release\qorx.exe context verify
.\target\release\qorx.exe security attest
```

If this gate fails, do not publish the claim.
