# Qorx Performance Notes

Qorx performance claims must be measured from the current checkout.

## Local Accounting

Qorx estimates token mass with:

```text
estimated_tokens = ceil(character_count / 4)
```

Redshift is:

```text
redshift = local_estimated_tokens / visible_estimated_tokens
```

This is local accounting. It is not a provider tokenizer and not a provider
invoice.

## Required Measurement

```powershell
cargo build --release
.\target\release\qorx.exe index .
.\target\release\qorx.exe session
.\target\release\qorx.exe bench --budget-tokens 900 "resolver boundary proof"
```

Publish the command, date, checkout, and output if you publish numbers.

## Rule

Token omission is not the same as task quality. A serious benchmark must also
measure retrieval correctness, support rate, latency, and task success.
