# Qorx Operations

## Build

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Index

```powershell
qorx index .
qorx session
```

## Verify

```powershell
qorx context snapshot
qorx context verify
qorx security attest
```

## Run Source

```powershell
qorx qorx .\goal.qorx
```

## Compile Source

```powershell
qorx qorx-compile .\goal.qorx --out .\goal.qorxb
qorx qorx .\goal.qorxb
```

## Publish Discipline

Do not publish a release from a dirty tree. Do not publish a claim whose proof
command is stale. Do not publish a standards claim before the registration
exists.
