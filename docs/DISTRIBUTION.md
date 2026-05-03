# Distribution boundary

Qorx Community Edition is source-first.

This public repository no longer publishes or documents convenience
distribution channels from `main`.

## CE distribution

Supported public path:

```sh
cargo install --git https://github.com/bbrainfuckk/qorx --branch main --locked qorx
```

Or clone the repo and build:

```sh
git clone https://github.com/bbrainfuckk/qorx.git
cd qorx
cargo test
cargo build --release
```

## Not published from CE

- GitHub release binaries.
- npm package.
- PyPI package.
- WinGet manifest.
- Scoop bucket.
- Homebrew tap.
- Snap package.
- AUR, Debian, RPM, or Nix recipes.
- Docker image.
- signed installers.
- auto-update channel.

Those channels are reserved for Qorx Local Pro or maintainer-controlled product
distribution.

## Maintainer note

Historical tags and archives may still exist. Do not treat them as the current
official product surface. New public work should point to Community Edition and
the commercial Qorx Local Pro boundary.
