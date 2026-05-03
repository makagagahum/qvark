# Qorx Formulas

Version: 1.0.4

## Local Mass

```text
mass(text) = ceil(len_utf8_chars(text) / 4)
```

This is Qorx's deterministic local estimate.

## Redshift

```text
redshift = baseline_local_mass / visible_carrier_mass
```

## Baseline-to-Compact

```text
omitted_mass = baseline_local_mass - visible_carrier_mass
```

## B2C Quant Allocation

```text
coverage = matched_query_terms / query_terms
expected_value = ln(1 + retrieval_score) * 12
               + coverage * 48
               + symbol_bonus
               + structural_bonus
token_cost = sqrt(quark_tokens) * 1.7
omission_risk = clamp(1 - coverage, 0.05, 0.95)
cache_value = min(ln(1 + quark_tokens), 8) for stable quarks

net = expected_value + cache_value
      - token_cost
      - redundancy_penalty
      - omission_risk * 9
```

Selection is greedy by positive `net / needed_tokens` under the requested token
budget. Redundancy increases when candidates share a path, parent directory,
symbols, matched terms, or sparse-vector overlap.

This is an engineering heuristic for local context selection. It is not a claim
about finance or consumer behavior by itself.

## Estimated USD

```text
estimated_saved_usd = omitted_input_tokens * usd_per_input_token
```

This estimate is only valid when `usd_per_input_token` is explicitly declared.
It is not proof of provider invoice savings.

## Support Rate

```text
support_rate = supported_claims / total_claims
```

A supported claim has cited local evidence. An unsupported claim should be
refused or marked unsupported.

## Task Result

Token reduction alone is incomplete. A useful evaluation also measures:

- support rate;
- retrieval correctness;
- latency;
- task success;
- actual routed provider cost when a billing claim is made.
