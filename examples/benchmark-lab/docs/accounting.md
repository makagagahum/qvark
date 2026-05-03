# Local accounting

Qorx benchmark reports use deterministic local token estimates. The default
estimator is `ceil(chars / 4)`.

Redshift is local accounting:

```text
redshift = indexed_local_tokens / visible_tokens
```

Baseline-to-Compact accounting compares the indexed local baseline with a
smaller carrier, proof page, pack, or squeeze output:

```text
omitted_tokens = indexed_local_tokens - visible_tokens
```

These numbers are not provider invoices. Provider invoice savings require routed
provider billing evidence.
