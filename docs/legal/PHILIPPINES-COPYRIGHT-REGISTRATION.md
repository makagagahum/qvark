# Philippines Copyright Registration Packet

This file is a practical filing packet for Qorx in the Philippines. It is project documentation, not legal advice.

## Current Status

Qorx already has automatic copyright protection as soon as the code, docs, article, and assets were written. The repository also has public timestamp evidence through Git commits, GitHub release metadata, Zenodo DOI records, citation metadata, and Software Heritage-style archival.

Official Philippine copyright registration or deposit is still a separate author action. It requires Marvin Sarreal Villanueva to log in, certify the submission, provide identity details, upload copies, and pay any applicable government fees.

## What To Register

Register these as separate works if the portal allows separate records:

| Work | Suggested category | Upload package |
| --- | --- | --- |
| Qorx source code | Computer program / software work | Public source archive from the GitHub release or Zenodo archive. Do not upload local secrets, `.env`, signing seeds, provider tokens, RAM-drive state, or private `qorx-data`. |
| Qorx article | Literary/scientific article or written work | `docs/papers/qorx-ai-language-paper.md`, preferably converted to PDF, plus `docs/papers/ARTICLE-LICENSE.md`. |
| Qorx icon/logo artwork | Artistic work, if original | `docs/assets/qorx-icon.png` and source artwork, if registering marks/assets separately. |

## Author And Rights Metadata

Use this metadata consistently:

- Author: Marvin Sarreal Villanueva
- Copyright owner: Marvin Sarreal Villanueva
- Location: Metro Manila, Philippines
- Public contact: marvin@orin.work
- Alternate contact: msarvillan@gmail.com
- Project title: Qorx
- Article title: Qorx: A Language and Runtime for Local Context Resolution
- Year of creation/publication: 2026
- Public release date: 2026-05-01
- Repository: https://github.com/bbrainfuckk/qorx
- Zenodo all-versions DOI: https://doi.org/10.5281/zenodo.19875352
- Latest checked archived release DOI: https://doi.org/10.5281/zenodo.19907103
- Q-LCR preprint DOI: https://doi.org/10.5281/zenodo.19953308
- Code license: AGPL-3.0-only
- Article license: CC BY-NC-ND 4.0
- Q-LCR preprint Zenodo license: CC BY-ND 4.0

## IPOPHL Filing Path

Primary channel: IPOPHL Copyright Registration and Deposit System (CORDS).

Before filing:

1. Prepare a clean source archive from the public release.
2. Prepare the article as PDF if the portal requires a document upload.
3. Prepare a valid government ID for the author/owner.
4. Keep `LICENSE`, `NOTICE`, `CITATION.cff`, `TRADEMARKS.md`, and `docs/papers/ARTICLE-LICENSE.md` in the evidence folder.
5. Do not include private keys, provider credentials, local memory files, local `qorx-data`, or personal documents unrelated to the copyrighted work.

Suggested description:

```text
Qorx is a small domain-specific language and local runtime for context resolution. It defines .qorx source, .qorxb protobuf bytecode, qorx:// handles, local state, proof paging, named resolver steps, and deterministic local accounting.
```

Registration boundary:

- This registers/deposits copyright evidence for the submitted expression.
- It does not create a patent.
- It does not stop independent implementations of the same broad idea.
- It does not replace trademark registration for the Qorx name or logo.

Official sources:

- IPOPHL copyright page: https://www.ipophil.gov.ph/copyright/
- IPOPHL Copyright Registration and Deposit System page: https://www.ipophil.gov.ph/copyright-registration-and-deposit-system/
- WIPO copyright FAQ: https://www.wipo.int/en/web/copyright/faq-copyright

## National Library / Legal Deposit Boundary

If Qorx's article is later published as a formal book, monograph, e-book, or periodical issue in the Philippines, check whether National Library of the Philippines legal deposit applies to that published material.

Legal deposit is not the same thing as IPOPHL copyright registration. Treat it as library deposit and publication compliance, not as a patent or ownership grant.

Official source:

- National Library of the Philippines: https://web.nlp.gov.ph/

## Evidence Folder Checklist

Create a local upload folder outside the repo, for example:

```text
Qorx-IPOPHL-Submission/
  01-source/
    qorx-v1.0.0-public-source.zip
  02-article/
    qorx-ai-language-paper.pdf
    ARTICLE-LICENSE.md
  03-evidence/
    LICENSE
    NOTICE
    CITATION.cff
    TRADEMARKS.md
    IP-PROTECTION.md
    Zenodo-DOI.txt
    GitHub-Release-URL.txt
  04-private-not-uploaded/
    government-id-copy
    portal-receipts
```

Keep the private identity and receipt files out of Git.
