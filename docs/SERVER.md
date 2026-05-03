# Server boundary

The public Community Edition does not ship the official background runtime as a
supported product surface.

In this repo, the daemon/proxy implementation is not shipped in current `main`.
The CE command line still reserves and refuses the public daemon commands:

```text
qorx daemon
qorx daemon run
qorx daemon start
qorx daemon status
qorx daemon stop
```

Those commands are part of Qorx Local Pro.

## Why

The daemon is the product layer that makes Qorx feel alive on a machine. It
controls the local HTTP gateway, provider routing, workstation startup, tray UX,
and tool integration path. Shipping that as a public convenience surface would
make Community Edition look like the paid local product.

## Community path

Use source-build CLI commands instead:

```sh
cargo build --release
./target/release/qorx doctor --json
./target/release/qorx index .
./target/release/qorx strict-answer "what proves Qorx is a language runtime?"
```

## Product path

Use Qorx Local Pro for:

- local daemon management.
- Windows tray.
- provider proxy routing.
- auto-start.
- signed installer.
- auto-update.
- account activation.
- cloud capsule sync.
- team policy.

## Network boundary

Do not expose self-built Qorx gateway experiments to untrusted networks. Any
shared deployment needs authentication, TLS, rate limits, logs, backups, and a
clear data-retention policy. The hosted Qorx API is the product path for that
surface.
