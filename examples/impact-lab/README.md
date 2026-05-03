# Qorx Impact Lab

This is a tiny deterministic fixture app for proving `qorx impact` before using it on a real repository.

```powershell
qorx.exe index .
qorx.exe impact "login session behavior" --diff-file examples\impact-lab\login.diff --budget-tokens 1200
```

Expected shape:

- `src/routes/auth.ts` is the changed path.
- `src/services/session.ts` and `src/services/audit.ts` are callees.
- `tests/auth.test.ts` is a caller/test.
- `src/services/billing.ts` is unrelated and should not be selected unless the query asks for billing.
