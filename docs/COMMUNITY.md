# Qorx Community Edition

This repository is Qorx Community Edition.

Community Edition exists so people can inspect the source, build the CLI, test
the language/runtime model, and verify the research claims. It is not the
official paid local product.

## Included

- AGPL source code.
- `.qorx` source parsing.
- `.qorxb` bytecode.
- local indexing and search.
- strict evidence answers.
- B2C accounting estimates.
- SAFE-R claim checks.
- source build instructions.

## Not included

- signed official installers.
- public release binaries.
- npm, PyPI, WinGet, Scoop, Homebrew, Snap, or Docker distribution.
- Windows tray UX.
- auto-update.
- daemon autostart.
- provider proxy routing.
- one-click Codex, Antigravity, Claude, Gemini, or other CLI activation.
- hosted Qorx API account features.
- cloud capsule sync.
- team policy and fleet controls.
- commercial support.

Those surfaces belong to Qorx Local Pro or the hosted Qorx API product.

## Current CE gate

The public CE binary refuses these commands:

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

The source still carries protocol and research code so the public record remains
auditable. The official product layer lives outside this repository.

## Forks

The AGPL license allows forks under its terms. The Qorx name, logo, official
release identity, and product marks are not granted for confusing use. A fork
must not imply that it is the official Qorx distribution.

## Commercial line

Qorx Local Pro is the supported local product:

- signed installer.
- local tray and updater.
- account activation.
- managed local vault UX.
- provider and CLI integration.
- cloud capsule sync when enabled by the user.
- team controls and support.

Community Edition is for source review and experimentation. Local Pro is the
product customers should install.
