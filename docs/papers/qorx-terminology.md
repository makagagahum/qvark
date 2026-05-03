# Qorx terminology

Status: public terminology note, not peer reviewed.

Version: 0.2

Date: May 2, 2026

Author: Marvin Sarreal Villanueva

## Purpose

This note defines the terms Qorx uses for local context resolution in AI and
developer tooling workflows. The plain workflow comes first: carry a small
object, resolve local evidence, cite it, or refuse.

The phrase "context resolution" already exists in other fields and systems. Qorx
does not claim to own the generic phrase. This note coins the Qorx-specific
terms below and gives them a public timestamp.

## Core Terms

### Qorx Local Context Resolution

Qorx Local Context Resolution, or Q-LCR, is the runtime process of resolving a
small carrier into local evidence under a declared budget and mode.

A carrier can be:

- a `.qorx` source file;
- a `.qorxb` bytecode file;
- a `qorx://` handle;
- a compact evidence pack.

The resolver must be available. A remote model does not learn hidden local data
from a handle alone.

### Carrier

A carrier is the small object that moves through a workflow. It names the mode,
question, budget, handle, or bytecode needed for resolution.

Short name: `phot`.

### Resolver boundary

The resolver boundary is the line between model-visible text and local state.
Qorx treats that boundary as explicit. Data on the local side must be resolved,
packed, cited, or refused.

Short name: `hzon`.

### Evidence quark

An evidence quark is a bounded, hashed, token-estimated evidence chunk. Qorx
uses quarks to build compact proof pages without treating summaries as raw
source.

Short name: `qrk`.

### Proof page

A proof page is the model-visible evidence returned by a resolver. It should
contain enough local evidence for the task and enough citation data to audit the
answer.

### qshf factor

The qshf factor is the ratio between local context mass and visible carrier
mass:

```text
qshf = local_estimated_tokens / visible_estimated_tokens
```

`qshf` is local accounting. It is not a provider invoice claim.

Short name: `qshf`.

### Baseline-to-Compact

Baseline-to-Compact, or B2C, is the local accounting method for comparing a
baseline context payload with a smaller carrier or evidence pack.

Short name: `b2c`.

## Short Name Table

The short names are optional labels for logs, UI, and compact docs. They do not
replace the plain terms in first-use explanations.

| Long term | Short name | Meaning |
| --- | --- | --- |
| `.qorx` source | `qwav` | Human-readable source program. |
| compile output | `qfal` | Protobuf-envelope bytecode or execution plan. |
| carrier | `phot` | Small model-visible object. |
| evidence quark | `qrk` | Bounded evidence chunk. |
| local state | `qosm` | Local Qorx state store. |
| resolver boundary | `hzon` | Boundary between local state and model-visible text. |
| qshf factor | `qshf` | Local baseline-to-visible reduction ratio. |
| session handle | `qses` | `qorx://s/...` reference. |
| capsule handle | `qcap` | `qorx://c/...` context bundle reference. |
| evidence pack | `qpak` | Ranked local evidence bundle. |
| B2C allocator | `qalc` | Budgeted quark selector. |

## Optional Internal Terms

These are useful for implementation notes, but they should not be front-loaded
in public onboarding copy.

| Term | Meaning |
| --- | --- |
| `qfld` | Active resolution space or index being searched. |
| `qspn` | Local priority signal for a quark during selection. |
| `qgrv` | Weight assigned by relevance scoring. |
| `qflx` | Change in local context mass between sessions. |
| `qfus` | Merge two capsules or evidence packs into one bundle. |
| `qv0d` | Resolver miss: no local evidence supports the request. |

Legacy aliases may appear in old notes: `redshift` maps to `qshf`, and
`qvoid` maps to `qv0d`.

## Boundary

These terms describe Qorx's implementation and evaluation vocabulary. They do
not create a standard by themselves. A standard would require independent use,
outside review, and a separate registration or standards process.

The physics-inspired names are metaphors over runtime objects. They are not
physics claims.

