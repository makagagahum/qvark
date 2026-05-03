# Installing Qorx Community Edition

Community Edition is source-first. Build it with Rust and Cargo.

## Source build

```sh
git clone https://github.com/bbrainfuckk/qorx.git
cd qorx
cargo test
cargo build --release
./target/release/qorx --version
```

Windows:

```powershell
git clone https://github.com/bbrainfuckk/qorx.git
cd qorx
cargo test
cargo build --release
.\target\release\qorx.exe --version
```

## Cargo git install

Install from the current public source branch:

```sh
cargo install --git https://github.com/bbrainfuckk/qorx --branch main --locked qorx
qorx --version
```

## Distribution boundary

This repository no longer ships public convenience binaries or package-manager
wrappers from `main`.

Not included in Community Edition:

- GitHub release zips and tarballs.
- npm and PyPI wrappers.
- WinGet, Scoop, Homebrew, Snap, AUR, Debian, RPM, Nix, or Docker distribution.
- signed Windows installers.
- tray or auto-update packaging.

Those are official product surfaces and belong to Qorx Local Pro or another
maintainer-controlled release channel.

## What to run

Use the CE command set after building:

```powershell
.\target\release\qorx.exe doctor --json
.\target\release\qorx.exe index .
.\target\release\qorx.exe strict-answer "what proves Qorx is a language runtime?"
.\target\release\qorx.exe b2c-plan "what proves Qorx is a language runtime?" --budget-tokens 900
.\target\release\qorx.exe security attest
```

Read [Community Edition](COMMUNITY.md) before treating a self-built binary as an
official Qorx product.
