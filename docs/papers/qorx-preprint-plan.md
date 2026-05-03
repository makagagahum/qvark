# Qorx preprint and paper plan

Status: working plan.

## Goal

Publish Qorx terms and claims in a citable form without overstating review
status.

The safe wording is:

```text
Qorx Local Context Resolution (Q-LCR) is defined in the Qorx technical preprint.
```

Do not say external review has proved Qorx. That requires external evaluation.

## Route

### 1. Technical report in the repository

Already possible. The repo is timestamped by GitHub, release tags, Zenodo, and
Software Heritage style archival.

Artifacts:

- `qorx-terminology.md`
- `qorx-local-context-resolution-preprint.md`
- `dist/qorx-local-context-resolution-preprint.pdf`
- `ZENODO-PREPRINT-UPLOAD.md`
- `zenodo-preprint-metadata.json`
- `qorx-evidence-map.md`
- `qorx-scientific-formulas.md`

Current status: the preprint is published on Zenodo.

Preprint DOI:

```text
10.5281/zenodo.19953308
```

Record:

```text
https://doi.org/10.5281/zenodo.19953308
```

### 2. Zenodo DOI

Zenodo is the fastest DOI path for a technical report. Zenodo assigns a DOI when
the record is published.

Done for version 0.2:

```text
https://doi.org/10.5281/zenodo.19953308
```

Suggested record type:

```text
Publication / Preprint
```

Suggested title:

```text
Qorx Local Context Resolution: a handle-resolved runtime model for local AI context
```

Suggested keywords:

```text
Qorx; local context resolution; retrieval augmented generation; agent memory;
Rust; bytecode; provenance; AI tooling
```

### 3. OSF Preprints

OSF Preprints can host preprints with DOI and persistent URLs after moderation.
Use it if you want a more paper-like preprint record.

### 4. arXiv later

Do not rush arXiv with a vocabulary-only note. Use arXiv only after the paper has
an empirical evaluation and a stronger related-work section.
Independent researchers may also need endorsement depending on category and
account history.

Likely categories after evaluation:

- `cs.SE` for software engineering;
- `cs.CL` only if the paper evaluates language-model behavior;
- `cs.AI` only if the contribution is clearly AI-method related.

### 5. JOSS later

The Journal of Open Source Software is a good target if Qorx becomes research
software used by others. JOSS review is public on GitHub and focuses on the
software, documentation, tests, and statement of need.

## Required before serious submission

- PDF manuscript. Done for the Zenodo route.
- Clear statement of need.
- Related work with citations.
- Installation instructions that work from a clean environment.
- Benchmarks over multiple repositories.
- Data or scripts for reproducing the evaluation.
- Honest limitations section.
- No claims of peer review before peer review.

## Sources for submission routes

- Zenodo quick start: https://help.zenodo.org/docs/get-started/quickstart/
- OSF Preprints guide: https://help.osf.io/article/376-preprints-home-page
- JOSS: https://joss.theoj.org/
