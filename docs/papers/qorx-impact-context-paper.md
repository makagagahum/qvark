# Qorx Impact Context

Qorx impact analysis is about changed evidence, not broad file dumping.

## Model

Given a task or diff, Qorx should identify:

- changed paths;
- symbols near the change;
- related local evidence;
- tests or routes that may be affected;
- unsupported claims.

## Command

```powershell
qorx impact "runtime behavior" --budget-tokens 1200
qorx map "runtime behavior" --budget-tokens 900
```

## Boundary

Impact context is a local retrieval aid. It does not prove the final patch is
correct. Use tests, review, and runtime checks.
