# Qorx Language

Qorx source is plain text. It is intentionally small: a domain-specific language
for AI context workflows with variables, named resolver steps, semantic checks,
branches, assertions, QIR, protobuf-envelope bytecode, and a local interpreter.

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

## Grammar

This is the public grammar for version 1:

```ebnf
program      = header, { directive | statement } ;
header       = "QORX", space, version, newline ;
version      = "1" ;
directive    = at_directive | colon_directive ;
at_directive = "@", key, space, value, newline ;
colon_directive = key, ":", space, value, newline ;
key          = "mode" | "ask" | "question" | "goal" | "objective"
             | "prompt" | "handle" | "session" | "capsule"
             | "budget" | "budget-tokens" | "limit" ;
statement    = import | binding | resolver_step | cache_policy | assertion | branch | emit ;
import       = "use", space, module, [ space, "as", space, ident ] ;
binding      = "let", space, ident, space, "=", space, string ;
resolver_step = op, space, ident, space, "from", space, ident,
                [ space, "budget", space, integer ],
                [ space, "limit", space, integer ] ;
cache_policy = "cache", space, ident, space, "key", space, ident,
               space, "ttl", space, integer ;
assertion    = "assert", space, "supported", "(", ident, ")" ;
branch       = "if", space, "supported", "(", ident, ")", space,
               "then", space, "emit", space, ident, space,
               "else", space, "emit", space, ident ;
emit         = "emit", space, ident ;
op           = "pack" | "strict" | "strict-answer" | "squeeze"
             | "map" | "cache-plan" | "session" ;
ident        = letter, { letter | digit | "_" | "-" } ;
module       = ident, { ".", ident } ;
string       = '"', { ? utf-8 text, with \" \\ \n \t escapes ? }, '"' ;
value        = ? utf-8 text without trailing newline ? ;
```

## Program Form

The language form is for AI-to-AI context packets. It lets agents pass a tiny
program or compiled `.qorxb` file instead of a pasted RAG chunk.

```text
QORX 1
use std.evidence
use std.branch as br
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
pack evidence from question budget 600
cache evidence key question ttl 3600
strict answer from evidence limit 1
if supported(answer) then emit answer else emit fallback
```

Execution is local:

1. `let` binds a string.
2. `pack` selects a budgeted local evidence pack.
3. `cache evidence key question ttl 3600` binds the stable step result to a deterministic local cache key.
4. `strict` answers only from local evidence.
5. `if supported(answer) then emit answer else emit fallback` chooses a result
   based on resolver evidence.
6. `assert supported(answer)` is still available when the program should fail
   closed instead of returning a fallback.
7. `emit` selects a result directly when no branch is needed.

The compiled bytecode contains canonical opcodes such as `BIND`, `PACK`,
`CACHE_BIND`, `STRICT_ANSWER`, `IF_SUPPORTED`, `THEN_EMIT`, `ELSE_EMIT`,
`ASSERT_SUPPORTED`, and `EMIT`. The compiler report also includes an AST and
QIR instructions such as `BIND_CONST`, `CALL_PACK`, `CALL_STRICT_ANSWER`, and
`IF_SUPPORTED`. The bytecode carries hashes for the program, AST, QIR, and
opcode stream. The runtime rejects bytecode when those streams no longer match
the compiled program.

The bytecode also carries `qstk`, a Forth-inspired stack tape. It is a compact
word stream for local dispatch, with words such as `use`, `lit`, `bind`, `src`,
`bud`, `lim`, `call`, `qif`, `then`, `qels`, and `emit`. It is not a Forth
compatibility claim; it gives Qorx a tiny deterministic stack-machine shape
inside the protobuf envelope.

The cache policy is a Qorx runtime policy. It does not claim GPU attention,
provider KV-cache, or hardware SRAM behavior. It gives Qorx a stable local key
for replayable resolver outputs and telemetry.

## Checking

`qorx-check` parses source and runs semantic checks without executing resolver
calls.

```powershell
qorx qorx-check .\goal.qorx
```

The checker rejects undefined symbols before compile. For example,
`strict answer from missing limit 1` fails at check/compile time instead of
waiting for runtime.

## Modes

| Mode | Result |
| --- | --- |
| `program` | Run named resolver steps and emit one result. |
| `strict-answer` | Cited answer from local evidence or refusal. |
| `pack` | Ranked evidence bundle. |
| `squeeze` | Query-focused line extraction. |
| `map` | Path, symbol, and relation map. |
| `cache-plan` | Stable/dynamic prompt split. |
| `session` | Compact session handle. |
| `agent` | Deterministic local plan. |

## Bytecode

`.qorxb` is protobuf-envelope bytecode. It is not a prompt. It is the compiled
form of the same source intent with canonical opcodes, a `qstk` stack tape,
instruction counts, and integrity hashes for local runtime dispatch.

```powershell
qorx qorx-compile .\goal.qorx --out .\goal.qorxb
qorx qorx-inspect .\goal.qorxb
qorx qorx .\goal.qorxb
```

## Compatibility

Version 1 readers must accept both `@key value` and `key: value` directive
forms. Unknown directives must not silently change execution semantics.
