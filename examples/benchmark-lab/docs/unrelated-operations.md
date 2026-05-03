# Unrelated operations notes

These notes exist to make the local index larger than the evidence needed for a
single query.

The example application keeps an audit log, rotates local build artifacts, and
stores release notes beside the source tree. A maintainer may ask about these
files later, but they are unrelated to Qorx Local Context Resolution.

The staging checklist includes linting, unit tests, package metadata checks,
readme rendering, and changelog review. The checklist is normal operational
context. It should not appear in a proof page about Q-LCR unless the query asks
about release operations.

The deployment notes mention service ownership, local log retention, health
checks, and rollback criteria. They do not define `.qorx`, `.qorxb`,
`qorx://`, proof pages, Redshift, or strict answer refusal.

The support runbook says to keep incident notes short, cite the failing command,
and avoid claiming a root cause until local evidence is available.
