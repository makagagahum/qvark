# Technical credibility

This page is for skeptical engineers.

## What Qorx is

Qorx is a small domain-specific language, compiler, and local runtime for AI
context workflows. It lets a workflow carry a checked `.qorx` program, compiled
`.qorxb` bytecode, or a `qorx://` handle instead of repeatedly pasting the same
local files into a prompt.

The implementation is Rust. The current compiler path parses `.qorx`, runs
semantic checks, lowers to AST and QIR, emits canonical opcodes, emits `qstk`,
and stores the result in a protobuf envelope.

## What Qorx is not

- Qorx is not a general-purpose language like Rust, Python, C, or Forth.
- Qorx is not Forth-compatible. `qstk` is only a small stack tape inside Qorx
  bytecode.
- Qorx is not universal compression.
- Qorx does not make remote models know hidden local files.
- Do not claim provider invoice savings without routed provider billing
  evidence.
- Do not claim task-quality gains from token reduction alone.

## Why this can still matter

Most AI coding workflows move too much text through prompts. Qorx treats local
context as addressable state. The visible message can be small because the
resolver can fault in evidence locally and return proof pages when needed.

That is the whole claim. It is useful only when the resolver exists and the
local state is indexed.

## Evidence to show

Use this order when presenting Qorx to a senior technical reviewer:

1. Show the source file.
2. Run `qorx qorx-check examples/goal.qorx`.
3. Run `qorx qorx-compile examples/goal.qorx --out target/goal.qorxb`.
4. Run `qorx qorx-inspect target/goal.qorxb`.
5. Point to AST, QIR, opcodes, `qstk`, and their hashes.
6. Run `qorx target/goal.qorxb`.
7. Show that provider calls are `0` for local resolver execution.
8. Run `scripts/safer-check.ps1`.
9. Run `scripts/check-testsprite-enterprise.ps1`.

## Questions reviewers should ask

- Is the language boundary narrow enough?
- Are bytecode integrity checks strict enough?
- Does `qstk` add real dispatch value or only another representation?
- Does the resolver return the right evidence under adversarial queries?
- What happens when the receiver has no resolver?
- What load data exists for the public API layer?

The honest answer today: Qorx is ready as a local CLI/runtime and internal
service component. A public multi-tenant SaaS still needs auth, tenant
isolation, backups, monitoring, rate limits, and load data.
