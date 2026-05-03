# Qorx Benchmark Lab

This fixture is for repeatable Qorx evaluation runs.

It is not meant to look like a large production repository. It is a controlled
repository with known evidence for Qorx Local Context Resolution, known carrier
terms, and one unsupported claim that should be refused.

The benchmark runner indexes this directory, then measures:

- session pointer size against the indexed local baseline;
- pack and squeeze output size under a budget;
- strict answer support for a known local claim;
- strict answer refusal for a claim absent from the index;
- local provider-call count for the deterministic agent route.

The fixture intentionally contains unrelated operational notes so the index has
context that should remain local unless a query needs it.
