# Qorx AI-Native Language And Runtime

This file is kept as a stable entry point. The handbook lives in
[`docs/handbook`](handbook/README.md).

Qorx is a small domain-specific language for local context resolution. A `.qorx`
file can be a compact directive file or a named resolver program. It tells the
runtime what evidence is requested, what budget applies, which resolver steps to
run, how to branch on supported evidence, and which result to emit. A `.qorxb`
file is the checked protobuf-envelope form of that source, including AST, QIR,
canonical opcodes, a Forth-inspired `qstk` stack tape, and integrity hashes.

## Minimal Source

```text
QORX 1
use std.evidence
use std.branch as br
let question = "which files explain how Qorx keeps local evidence outside the model prompt?"
let fallback = "qv0d: local evidence does not support this answer"
pack evidence from question budget 700
cache evidence key question ttl 3600
strict answer from evidence limit 2
if supported(answer) then emit answer else emit fallback
```

```sh
qorx qorx-check goal.qorx
qorx qorx-compile goal.qorx --out goal.qorxb
qorx qorx goal.qorxb
```

## Directives

| Directive | Meaning |
| --- | --- |
| `QORX 1` | Language version. |
| `@mode` | Execution mode. |
| `@ask` | Evidence question for strict modes. |
| `@goal` | Objective for pack, map, squeeze, or agent modes. |
| `@handle` | Optional `qorx://` handle. |
| `@budget` | Evidence budget, estimated with Qorx local accounting. |

## Modes

| Mode | Purpose |
| --- | --- |
| `program` | Run named resolver steps and emit a selected result. |
| `strict-answer` | Answer only from indexed local evidence. |
| `b2c-plan` | Plan a budgeted local quark portfolio with quant-style scoring. |
| `pack` | Return a compact evidence pack under budget. |
| `squeeze` | Extract query-relevant lines with citations. |
| `map` | Map changed paths, symbols, and related evidence. |
| `cache-plan` | Split stable and dynamic prompt regions. |
| `session` | Emit a compact session handle. |
| `agent` | Run the deterministic local plan. |

## Runtime Terms

| Term | Short name | Meaning |
| --- | --- | --- |
| `.qorx` | qwav | Source program. |
| `.qorxb` | qfal | Protobuf-envelope bytecode. |
| QIR | qir | Lowered resolver-call representation used by the compiler. |
| stack tape | qstk | Forth-inspired bytecode word stream for tiny local dispatch. |
| cache policy | qcas | Source-level cache binding for stable local resolver outputs. |
| resolver step | qop | Named opcode over local Qorx state. |
| carrier | phot | Small model-visible source, bytecode, handle, or pack. |
| quark | qrk | Bounded, hashed evidence chunk. |
| local state | qosm | Local Qorx protobuf state. |
| resolver boundary | hzon | Local-vs-visible boundary. |
| qshf factor | qshf | Local context to visible carrier ratio. |
| B2C | b2c | Baseline-to-Compact accounting. |
| B2C allocator | qalc | Deterministic local selector for budgeted quark portfolios. |
| `qorx://s/...` | qses | Session handle. |
| `qorx://c/...` | qcap | Capsule handle. |
| `qorx://u/...` | qevt | Event handle. |
| `qorx://l/...` | qlat | Lattice handle. |
| `qorx://f/...` | qfed | File-share handle. |

## Boundary

Qorx handles are not magic strings. They work when the receiving workflow can
route them to a Qorx resolver. Without that resolver, they are just identifiers.

Read the full handbook:

- [Handbook](handbook/README.md)
- [Language](handbook/language.md)
- [Runtime](handbook/runtime.md)
- [Protocol](handbook/protocol.md)
