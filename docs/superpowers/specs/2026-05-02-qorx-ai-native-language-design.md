# Qorx AI-Native Language Design

## Goal

Make Qorx a small domain-specific programming language for AI context workflows,
not a general-purpose application language. The language should let agents
exchange small, typed context programs that compile to protobuf bytecode and
execute against local Qorx resolver state.

## Research Basis

- LLVM's Kaleidoscope tutorial uses the standard compiler shape: parse source
  into an AST, then generate a lower-level representation from that AST.
- WebAssembly's core spec separates structure, validation, and execution. Qorx
  should copy that discipline without adopting Wasm as the first target.
- Language Server Protocol and Tree-sitter matter later for editor tooling, but
  the first release needs a stable AST/interpreter/compiler surface before a
  full editor stack.

## Language Identity

Qorx is a small AI-context DSL and local runtime.

It is designed for:

- AI agents passing context to other AI agents.
- Local LLM workflows that need compact context handles instead of repeated RAG
  chunks.
- Auditable resolver execution with token accounting.
- Protobuf-backed bytecode that can be cached, passed, signed, or replayed.

It is not designed for:

- General-purpose app logic.
- Arbitrary file compression.
- Making a remote model know hidden local data without a resolver.
- Replacing RAG for every workload.

## v1.0.4 Language Slice

The first real slice adds named programming constructs while preserving the
existing directive format.

```text
QORX 1
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
pack evidence from question budget 600
cache evidence key question ttl 3600
strict answer from evidence limit 1
if supported(answer) then emit answer else emit fallback
```

Semantics:

- `let name = "value"` binds a local string.
- `pack name from source budget N` runs Qorx evidence packing.
- `cache target key source ttl N` binds a resolver result to a deterministic
  local runtime cache key.
- `strict name from source limit N` runs strict local answering.
- `squeeze`, `map`, `cache-plan`, and `session` are resolver steps with the
  same named-step shape.
- `assert supported(name)` fails closed when a strict result is unsupported.
- `if supported(name) then emit a else emit b` branches on resolver support.
- `emit name` chooses the program output.

The compiler emits AST nodes, QIR instructions, canonical bytecode opcodes, and
`qstk`, a Forth-inspired stack tape inside the protobuf envelope:

- `BIND`
- `CONST_SHA256`
- `PACK`
- `CACHE_BIND`
- `STRICT_ANSWER`
- `SQUEEZE`
- `MAP`
- `CACHE_PLAN`
- `SESSION`
- `IF_SUPPORTED`
- `THEN_EMIT`
- `ELSE_EMIT`
- `ASSERT_SUPPORTED`
- `EMIT`

The protobuf payload stores the full parsed program, not only opcodes, so the
runtime can audit and replay the program. It also carries hashes for the
program, AST, QIR, opcode stream, `qstk`, and instruction count. The runtime
rejects a `.qorxb` if those streams no longer match the compiled program.

`qstk` is not a claim that Qorx is Forth-compatible. It is the small
stack-machine shape Qorx uses for local AI-context dispatch: `use`, `lit`,
`bind`, `src`, `bud`, `lim`, `call`, `qif`, `then`, `qels`, and `emit`.

## Runtime Boundary

Qorx bytecode is fast because it carries compact resolver intent, not bulk
context. Execution still happens through local Qorx state. If a recipient lacks
the resolver or the referenced evidence, the correct behavior is refusal or a
request for more context.

## Next Slices

1. Add a stable binary protobuf schema for bytecode instead of the generic
   state envelope.
2. Add `qorx fmt` before editor/LSP work.
3. Add a small standard library of resolver ops for local LLM runtimes.
