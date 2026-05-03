# Strict refusal

The strict-answer route is extractive. It answers from indexed quark excerpts
and cites local evidence. If a claim is absent from the local index, the correct
behavior is refusal rather than a generated guess.

This fixture supports claims about Qorx Local Context Resolution, carriers,
resolver boundaries, proof pages, and local accounting. It does not support
claims about unrelated external systems, private credentials, or production
traffic outside the fixture.

Unsupported claims should return `not_found` with no answer bytes and no
evidence records.
