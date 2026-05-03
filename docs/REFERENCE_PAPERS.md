# Qorx Reference Papers And External Sources

Qorx is not built from vibes. The current repo keeps a local PaperQA corpus in
`research/papers`, plus official provider and provenance references in the docs.
This file is the readable map.

The papers support the architecture class around Qorx: retrieval-backed context
omission, prompt compression, repository memory, cache-aware request design,
agent memory, and runtime cache/KV boundaries. They do not prove that Qorx wins
on every task. Qorx-specific claims still need Qorx-specific benchmarks.

## Prompt Compression

| Reference | Local file or source | How it relates to Qorx |
| --- | --- | --- |
| LLMLingua | `research/papers/llmlingua_2310.05736.pdf` | Learned prompt compression supports the broader idea that prompts can be shortened while preserving useful information. Qorx core does not bundle LLMLingua. |
| LongLLMLingua | `research/papers/longllmlingua_2310.06839.pdf` | Long-context prompt compression and budget control. Qorx uses deterministic budgeted packing and extractive squeeze in the portable core. |
| Active Context Compression | `research/papers/active_context_compression_2601.07190.pdf` | Active context pruning supports the idea that not all context should remain visible all the time. |
| Gist Tokens | `research/papers/gist_tokens_2304.08467.pdf` | Soft-token memory is a model-side technique. Qorx treats this as adapter/future scope because vendor CLIs cannot consume arbitrary learned gist tokens. |
| Experience Compression Spectrum | `research/papers/experience_compression_spectrum_2604.15877.pdf` | Useful for thinking about compression levels and provenance loss. Qorx keeps exact local fallback instead of relying only on summaries. |

## Repository And Code Context

| Reference | Local file or source | How it relates to Qorx |
| --- | --- | --- |
| ReACC | `research/papers/reacc.pdf` / ACL 2022 source | Retrieval-augmented code completion supports bringing related code into the model context. Qorx supplies local code quarks to downstream agents. |
| Codebase-Memory | `research/papers/codebase_memory_2603.27277.pdf` | Codebase memory and graph-style context motivate Qorx's lightweight symbol and relation surfaces. |
| BM25 and lexical retrieval | referenced in the evidence map | Exact lexical retrieval remains a strong baseline. Qorx uses deterministic sparse terms plus path and symbol boosts. |
| Aider repository map | official aider docs | Repository maps are useful, but Qorx keeps a local quark store and budgeted evidence routes rather than only a static map. |

## Agent Memory And Long-Horizon Context

| Reference | Local file or source | How it relates to Qorx |
| --- | --- | --- |
| AgeMem | `research/papers/agemem_2601.01885.pdf` | Explicit memory operations matter for agents. Qorx exposes local memory CRUD and summaries. |
| AtomMem | `research/papers/atommem_2601.08323.pdf` | Atomized memory matches Qorx's quark-level evidence approach, but Qorx keeps its implementation deterministic. |
| Titans | `research/papers/titans_2501.00663.pdf` | Neural long-term memory is a real research direction. Qorx does not claim a Titans-like learned memory runtime. |
| TokMem | `research/papers/tokmem_2510.00444.pdf` | Token memory research informs future adapter ideas, not the current portable-core claim. |
| Memory Survey | `research/papers/memory_survey_2603.07670.pdf` | Broad survey background for explicit memory design. |
| Structural Memory | `research/papers/structural_memory_2412.15266.pdf` | Supports structured memory and provenance-aware state. |

## Hierarchical Memory

| Reference | Local file or source | How it relates to Qorx |
| --- | --- | --- |
| H-MEM | `research/papers/hmem_2507.22925.pdf` | Hierarchical memory supports multi-layer retrieval. Qorx implements deterministic lattice layers. |
| HiMem | `research/papers/himem_2601.06377.pdf` | Long-horizon memory organization. Qorx uses local mementos and raw-quark provenance. |
| TierMem | `research/papers/tiermem_2602.17913.pdf` | Provenance-aware tiered memory is close to Qorx's lattice/attestation boundary. |
| GAM | `research/papers/gam_2604.12285.pdf` | Graph-based agentic memory supports the idea of relations across memory nodes. Qorx keeps graph work lightweight in core. |

## Cache And Reuse

| Reference | Local file or source | How it relates to Qorx |
| --- | --- | --- |
| Preble | `research/papers/preble_2407.00023.pdf` | Prefix/cache-aware request design. Qorx has `cache-plan` for stable prefix and dynamic tail separation. |
| Don't Break the Cache | `research/papers/dont_break_cache_2601.06007.pdf` | Supports careful prompt structure so provider caches remain useful. |
| Similarity Caching | `research/papers/similarity_caching_1912.03888.pdf` | Approximate reuse can save cost but has correctness tradeoffs. Qorx keeps approximate answer replay out of the default path. |
| GPT Semantic Cache | `research/papers/gpt_semantic_cache_2411.05276.pdf` | Semantic caching supports future guarded adapters. Qorx ships exact replay cache first. |
| RAGCache | `research/papers/ragcache_2404.12457.pdf` | Retrieval cache design for RAG workflows. |
| Cache-Craft | `research/papers/cache_craft_2502.15734.pdf` | Chunk cache management for RAG. |
| Approximate Caching for RAG | `research/papers/approximate_caching_rag_2503.05530.pdf` | Approximate reuse is useful but must be measured and guarded. |
| Domain-Specific Semantic Cache | `research/papers/domain_specific_semantic_cache_2504.02268.pdf` | Domain-specific embeddings can improve cache reuse, but Qorx core avoids mandatory embedding runtimes. |
| vCache | `research/papers/vcache_2502.03771.pdf` | Verified semantic prompt caching supports the idea of cache correctness gates. |
| ContextPilot | `research/papers/contextpilot_2511.03475.pdf` | Long-context reuse. Qorx handles reuse through local handles and evidence resolution. |
| QVCache | `research/papers/qvcache_2602.02057.pdf` | Query-aware vector cache ideas inform future cache adapters. |

## Runtime And KV Boundaries

| Reference | Local file or source | How it relates to Qorx |
| --- | --- | --- |
| TurboQuant | `research/papers/turboquant_2504.19874.pdf` | KV/cache compression is a runtime measurement problem. Qorx can emit hints but does not claim realized TurboQuant/vLLM gains without a runtime proof. |
| vCache and QVCache | local cache papers above | Useful for guarded cache reuse and query-aware cache design. |

## Official Provider And Tooling References

| Source | URL | Qorx boundary |
| --- | --- | --- |
| OpenAI prompt caching | https://platform.openai.com/docs/guides/prompt-caching | Provider-side cache behavior is separate from Qorx local context omission. |
| Anthropic prompt caching | https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching | Qorx can help structure stable prefixes, but provider cache hits must be measured upstream. |
| Gemini context caching | https://ai.google.dev/gemini-api/docs/caching/ | Same provider-cache boundary. |
| Claude Code memory | https://docs.anthropic.com/en/docs/claude-code/memory | Memory files are useful, but Qorx adds a live local resolver/index path. |
| Gemini CLI `GEMINI.md` | https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/gemini-md.md | Context files are not the same as budgeted local evidence retrieval. |
| Cursor codebase indexing | https://docs.cursor.com/context/codebase-indexing | Cursor's server-backed indexing is a different deployment model. Qorx keeps the core local. |
| Cursor secure codebase indexing | https://cursor.com/blog/secure-codebase-indexing | Useful comparison for privacy and indexing boundaries. |

## Provenance, Signatures, And Storage

| Source | URL | Qorx boundary |
| --- | --- | --- |
| Protocol Buffers | https://protobuf.dev/ | Qorx uses protobuf-envelope persisted state and a typed context snapshot. |
| NIST FIPS 204 | https://csrc.nist.gov/pubs/fips/204/final | Qorx hybrid attestation uses post-quantum signature practice as a reference point. |
| C2PA Specification | https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html | Qorx provenance is local metadata, not a full embedded media manifest. |
| Microsoft kernel-mode signing requirements | https://learn.microsoft.com/en-us/windows-hardware/drivers/install/kernel-mode-code-signing-requirements--windows-vista-and-later- | Real RAM-drive drivers have OS/runtime boundaries. Qorx reports RAM mode separately from portable disk-backed mode. |

## PaperQA Result Boundary

PaperQA has been used here as a research audit path, not as an oracle. The local
corpus supports the architecture class. It does not by itself prove Qorx-specific
accuracy, latency, cost, or task-success improvement.

The next benchmark that matters is empirical: multiple repositories, repeated
agent tasks, routed provider traffic, retrieval-support scoring, latency, cache
hit rates, and invoice/provider-token comparison.
