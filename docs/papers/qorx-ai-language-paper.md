# Qorx: An AI-Native Language and Runtime for Local Context Resolution

Marvin Sarreal Villanueva
Metro Manila, Philippines
Version 1.0.4
May 2, 2026

## Abstract

Qorx is a small domain-specific language and local runtime for context
resolution. It defines `.qorx` source, `.qorxb` protobuf bytecode, `qorx://`
handles, named resolver steps, local state, proof paging, and
Baseline-to-Compact accounting. The core idea is simple: AI workflows should not
keep pasting the same local files into every prompt when a local resolver can
address, retrieve, and cite the needed evidence.

This paper describes the language boundary, runtime model, and measurable
claims. It is a technical report, not a peer-reviewed publication.

## 1. Problem

Large language model workflows repeatedly transmit the same local context:
source files, notes, logs, decisions, papers, and prior tool output. That
pattern is costly, slow, and hard to audit. It also blurs the boundary between
what a model actually received and what remained local.

Qorx treats context as local state with explicit addresses.

## 2. Language

A Qorx source file is small:

```text
QORX 1
@mode strict-answer
@ask which files explain how Qorx keeps local evidence outside the model prompt?
@budget 700
```

The runtime accepts source directives, compiles them to `.qorxb` when requested,
and executes a bounded local resolver plan.

## 3. Runtime

The runtime has five main surfaces:

1. local indexing into quarks;
2. `qorx://` session, capsule, event, lattice, and file-share handles;
3. proof routes such as strict answer, pack, squeeze, map, and impact;
4. Cosmos state for receipts, cache, provenance, and snapshots;
5. Redshift/B2C accounting for local context omitted from the visible carrier.

## 4. Claim

The primary claim is narrow:

> For context known to a local Qorx resolver, a workflow can carry a handle or
> compact evidence pack instead of resending all underlying context.

This is not a claim that an outside model knows hidden data. The resolver must
be available.

## 5. Measurement

Qorx reports local token estimates with `ceil(chars / 4)` unless another
tokenizer is explicitly used. Redshift is computed as:

```text
redshift = local_estimated_tokens / visible_estimated_tokens
```

B2C savings are local estimates:

```text
omitted_tokens = baseline_local_tokens - visible_carrier_tokens
```

Provider invoice savings require provider billing evidence. Qorx local
accounting is not a substitute for that evidence.

## 6. Boundary

Qorx does not claim:

- universal lossless compression of unknown data;
- provider invoice reduction without routed billing evidence;
- native remote-model support for `qorx://` handles;
- peer review;
- standards registration before registration exists.

## 7. Conclusion

Qorx defines a handle-first language/runtime for local context resolution. The
scientific value is in the boundary: addresses, proof pages, receipts, and
measured local accounting instead of vague context claims.
