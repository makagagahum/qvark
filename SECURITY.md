# Security Policy

Report security issues privately to the project owner before public disclosure.

Contact: marvin@orin.work. Alternate: msarvillan@gmail.com.

Do not include secrets, private keys, provider tokens, private corpora, or
machine-specific credentials in public issues.

Treat any pasted TestSprite key as compromised. Revoke or rotate it in the
TestSprite Web Portal, then store the replacement only as the GitHub Actions
secret `TESTSPRITE_API_KEY`. TestSprite secrets must not appear in workflow
YAML, docs, source files, screenshots, or release artifacts.

## Scope

In scope:

- local resolver state handling;
- bytecode parsing;
- handle expansion;
- provenance verification;
- cache behavior;
- path traversal or unsafe file access.

Out of scope:

- third-party model behavior;
- provider billing disputes;
- private forks not shared with the maintainer;
- unsupported local modifications.

## Boundary

Qorx stores local state. Treat exported capsules, shares, snapshots, and proof
pages as potentially sensitive.
