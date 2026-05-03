use serde::Serialize;

pub const RUN_HEADER_VALUE: &str = "qosm=core";
pub const STACK_ID: &str = "qshf_core";
pub const PROMPT_TAG: &str = "qosm=core qshf=core_b2c";
pub const HEADER_STAGES: &str = "capsule_pointer,session_pointer,b2c_quant_allocator,pack,squeeze,sparse_vectors,structural_signals,quark_compress,exact_replay_cache,provider_cache_accounting,kv_hints,usd_accounting";

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CostStage {
    pub name: &'static str,
    pub mode: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct AppliedScience {
    pub name: &'static str,
    pub proof: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostStackPolicy {
    pub run: &'static str,
    pub stack: &'static str,
    pub stages: &'static [CostStage],
    pub boundary: &'static str,
}

pub const STAGES: &[CostStage] = &[
    CostStage {
        name: "capsule_pointer",
        mode: "model-visible bulk context is replaced by a tiny qorx://c handle when a capsule is active",
    },
    CostStage {
        name: "session_pointer",
        mode: "repo memory is exposed as a tiny qorx://s handle instead of raw files",
    },
    CostStage {
        name: "b2c_quant_allocator",
        mode: "B2C-selected quarks are allocated with local bounded-budget scoring, redundancy penalties, omission-risk caps, and cache reuse value",
    },
    CostStage {
        name: "pack",
        mode: "the selected B2C quark portfolio is rendered as compact model-visible context",
    },
    CostStage {
        name: "squeeze",
        mode: "query-relevant lines are extracted locally before broad context is sent",
    },
    CostStage {
        name: "sparse_vectors",
        mode: "hashed lexical vectors score related quarks without dense model weights",
    },
    CostStage {
        name: "structural_signals",
        mode: "symbols and code-signal masks boost retrieval without parser packs",
    },
    CostStage {
        name: "quark_compress",
        mode: "routed JSON bodies deduplicate repeated large prompt blocks into local quark references",
    },
    CostStage {
        name: "exact_replay_cache",
        mode: "repeat non-streaming provider calls can be served locally without upstream spend",
    },
    CostStage {
        name: "provider_cache_accounting",
        mode: "OpenAI, Anthropic, and Gemini cached-token metadata is counted when providers return it",
    },
    CostStage {
        name: "kv_hints",
        mode: "safetensors-compatible KV hints are emitted for local runtime adapters",
    },
    CostStage {
        name: "usd_accounting",
        mode: "omitted, compressed, replayed, and provider-cached input tokens are priced as estimated savings",
    },
];

pub const APPLIED_SCIENCE: &[AppliedScience] = &[
    AppliedScience {
        name: "LLMLingua prompt compression principle",
        proof: "Qorx applies deterministic extractive squeeze/pack paths in core so low-rank evidence is omitted before provider calls; neural LLMLingua remains an optional external compressor",
    },
    AppliedScience {
        name: "LongLLMLingua position-aware context principle",
        proof: "Qorx applies token-budgeted ordering, session pointers, and exact retrieval handles so stable high-value context survives while bulk payload stays local",
    },
    AppliedScience {
        name: "Tree-sitter structural retrieval principle",
        proof: "Qorx applies in-core lexical symbol extraction plus code-signal masks for imports, routes, tests, errors, branches, assignments, calls, and config without bundling parser grammar packs",
    },
    AppliedScience {
        name: "Embedding retrieval principle",
        proof: "Qorx applies hashed sparse vectors and local similarity scoring in repo_index.pb; dense embedding models stay optional because bundling model weights would break the portable core",
    },
    AppliedScience {
        name: "ONNX compressor boundary",
        proof: "Qorx applies the compressor slot, env detection, and proof harness, while ONNX Runtime and model weights remain external so the release binary stays small and portable",
    },
    AppliedScience {
        name: "TurboQuant/vLLM KV boundary",
        proof: "Qorx emits safetensors-compatible KV hints and runtime target metadata for local adapters; this is not runtime KV tensor compression, which requires a real inference engine to own and measure the live KV cache",
    },
    AppliedScience {
        name: "semantic and RAG cache principle",
        proof: "Qorx applies exact replay caching, provider cached-token accounting, and quark-level retrieval reuse locally before spending model-visible tokens",
    },
    AppliedScience {
        name: "Quant portfolio allocation principle",
        proof: "Qorx applies bounded knapsack-style value density, portfolio diversification penalties, omission-risk caps, and stable-quark cache value inside b2c-plan and pack; it is deterministic local planning, not a financial-performance claim",
    },
    AppliedScience {
        name: "B2C qshf accounting",
        proof: "Qorx reports omitted input tokens, cached provider tokens, context_estimate_method=est=char4, and USD estimates so savings claims stay tied to measured local accounting",
    },
];

pub fn policy() -> CostStackPolicy {
    CostStackPolicy {
        run: RUN_HEADER_VALUE,
        stack: STACK_ID,
        stages: STAGES,
        boundary: "Qorx runs the portable qosm/qshf accounting stack in core. These are Qorx vocabulary labels for local protobuf state and baseline-to-compact ratios, not physics claims. Heavy ML/GPU runtimes activate only when installed; runtime KV tensor savings still require a local inference engine measurement.",
    }
}
