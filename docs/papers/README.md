# Qorx Papers

This directory contains public technical writing for Qorx.

Use the handbook as the implementation authority. Use papers for the argument,
not for unstated runtime behavior.

## Published Preprint

Qorx Local Context Resolution is published as a Zenodo preprint:

```text
Villanueva, Marvin Sarreal. Qorx Local Context Resolution: A Handle-Resolved
Runtime Model for Local AI Context. Zenodo, 2026.
DOI: 10.5281/zenodo.19953308
```

Record: https://doi.org/10.5281/zenodo.19953308

## Current Files

| File | Purpose |
| --- | --- |
| `qorx-ai-language-paper.md` | Technical paper for Qorx as a language/runtime. |
| `qorx-local-context-resolution-preprint.md` | Preprint draft defining Qorx Local Context Resolution. |
| `dist/qorx-local-context-resolution-preprint.pdf` | Uploadable PDF manuscript generated from the preprint. |
| `ZENODO-PREPRINT-UPLOAD.md` | Zenodo preprint record and upload fields. |
| `zenodo-preprint-metadata.json` | Zenodo metadata for the preprint record. |
| `qorx-terminology.md` | Coined Qorx terminology and boundary notes. |
| `qorx-preprint-plan.md` | DOI/preprint/journal route and submission checklist. |
| `qorx-evidence-map.md` | Claim-to-proof map. |
| `qorx-scientific-formulas.md` | Local accounting formulas. |
| `qorx-impact-context-paper.md` | Impact-context notes. |
| `ARTICLE-LICENSE.md` | Article license notice. |

## Rule

Do not imply peer review unless the paper has actually passed peer review. Do
not cite local Redshift accounting as provider invoice savings.

## Build the PDF

```powershell
python scripts/build-preprint-pdf.py
```
