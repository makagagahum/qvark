# Qorx IP Protection Status

This file summarizes the free protections currently used by Qorx. It is project documentation, not legal advice.

## Current Protection Stack

| Area | Current status | What it protects |
| --- | --- | --- |
| Source code | `AGPL-3.0-only` | Keeps code free/open and requires network-service source sharing under AGPL terms. |
| Operational docs | `AGPL-3.0-only` | Keeps build/run documentation under the same free-software terms as the code. |
| Scholarly article | `CC BY-NC-ND 4.0` | Allows sharing the unmodified article with attribution while blocking commercial redistribution and derivative article versions without permission. |
| Q-LCR preprint | `CC BY-ND 4.0` on Zenodo | Published preprint record for Qorx Local Context Resolution. DOI: `10.5281/zenodo.19953308`. |
| Citation and timestamp | `CITATION.cff`, Zenodo DOI, Software Heritage | Creates public authorship, release, and archival records. |
| Brand identity | `NOTICE` and `TRADEMARKS.md` | Reserves Qorx name, logos, marks, and project identity from confusing or endorsement-implying use. |
| Contributions | `CONTRIBUTING.md` and DCO sign-off | Requires contributors to certify they can license their contributions. |

## Philippine Registration Packet

For the practical Philippines filing checklist, use [`PHILIPPINES-COPYRIGHT-REGISTRATION.md`](PHILIPPINES-COPYRIGHT-REGISTRATION.md).

Copyright exists automatically, but IPOPHL registration/deposit is an account-bound author action. It requires Marvin Sarreal Villanueva to log in, submit identity details, upload the work, certify the submission, and pay any applicable government fee.

## Patent Boundary

Qorx is not patented by this repository. A patent requires a patent application filed with the relevant patent office and, ultimately, examination or grant depending on jurisdiction.

Public release, DOI archival, and Software Heritage archival can help establish public prior-art timestamps, but they do not create a patent and they do not replace legal patent advice.

## Why Qorx Does Not Use MIT By Default

MIT is permissive and useful for maximum adoption, but it gives other parties broad rights to reuse the code in closed products. Qorx uses `AGPL-3.0-only` because it is the stronger free/open-source protection for a network-facing AI gateway: if someone runs a modified Qorx as a service, the AGPL source-sharing requirements matter.

## What Is Not Protected For Free

- Patent rights are not created without a patent filing.
- Registered trademark rights are not created without a trademark process.
- Legal compliance claims such as HIPAA, SOC 2, or ISO certification require actual compliance work and audit evidence.
- A public repository cannot prevent all copying; it defines enforceable license terms and public attribution records.

## Practical Rule

Use this wording:

> Qorx is AGPL-licensed free software with separate scholarly writing licenses,
> public DOI archival, Software Heritage archival, DCO contribution terms, and
> reserved project marks. Qorx is not patented unless a separate patent filing
> is made.
