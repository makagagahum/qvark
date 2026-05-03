# Qorx Protocol

The protocol is handle-first.

```text
qorx://<kind>/<id>
```

## Kinds

| Kind | Form | Description |
| --- | --- | --- |
| Session | `qorx://s/<fingerprint>` | Current indexed workspace state. |
| Capsule | `qorx://c/<fingerprint>` | Folder or corpus bundle. |
| Event | `qorx://u/<fingerprint>` | Local action receipt. |
| Lattice | `qorx://l/<fingerprint>` | Memory/provenance state. |
| File share | `qorx://f/<fingerprint>` | Export/import state. |

## Resolver Contract

A resolver response should identify:

- handle;
- query or objective;
- evidence pages;
- paths and line ranges when applicable;
- hashes or fingerprints;
- budget used;
- boundary notes.

## Media Types

Public registrations are future work. Until then, use these descriptive labels:

| Surface | Label |
| --- | --- |
| Qorx source | `text/qorx` |
| Qorx bytecode | `application/qorxb` |
| Qorx handle | `text/uri-list` entry containing `qorx://...` |

Do not claim IANA registration until it exists.
