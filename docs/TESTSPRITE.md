# TestSprite QA

TestSprite is not required to build or use Qorx Community Edition.

For this public repo, the TestSprite check is a documentation and workflow
hygiene gate. It verifies that the repo points users to the Community Edition
boundary and does not expose a literal TestSprite secret.

For the hosted Qorx API or Qorx Local Pro onboarding site, keep a separate
TestSprite suite against a public staging URL.

## Secret

If a TestSprite key was pasted into chat, public logs, an issue, or a commit,
revoke or rotate it in the TestSprite Web Portal before using it again.

Store a replacement only as a GitHub Actions secret:

```text
TESTSPRITE_API_KEY
```

Do not put the key in `.env`, workflow YAML, docs, source files, release notes,
or screenshots.

## Workflow

The public workflow is:

```text
TestSprite Enterprise QA
```

It is manually run with:

- `base_url`: public staging URL for a product surface.
- `blocking`: `true` to fail the workflow when TestSprite reports a failing suite.

Community Edition does not use TestSprite to prove local daemon or tray behavior.
Those surfaces are Pro-only.

## Local command

Before relying on the cloud gate, verify the repo wiring locally:

```powershell
.\scripts\check-testsprite-enterprise.ps1
```

This check verifies the secret policy, public URL requirement, blocking mode,
Community Edition boundary docs, and the absence of literal TestSprite-style
secrets.
