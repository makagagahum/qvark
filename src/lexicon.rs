use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize)]
pub struct LexiconReport {
    pub schema: String,
    pub language: String,
    pub format: String,
    pub vocabulary: Value,
    pub aliases: Value,
    pub terms: &'static [LexiconTerm],
    pub layers: Value,
    pub boundary: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct LexiconTerm {
    pub name: &'static str,
    pub kind: &'static str,
    pub meaning: &'static str,
}

pub const TERMS: &[LexiconTerm] = &[
    term("qrk", "data", "bounded hashed evidence chunk"),
    term("qosm", "runtime", "local protobuf resolver state"),
    term("qwav", "language", ".qorx source before compile"),
    term("qfal", "language", ".qorxb compiled bytecode"),
    term("phot", "carrier", "tiny model-visible carrier"),
    term("hzon", "boundary", "local versus model-visible boundary"),
    term("qshf", "accounting", "local baseline-to-compact ratio"),
    term("b2c", "accounting", "baseline-to-compact token accounting"),
    term("qir", "compiler", "Qorx intermediate representation"),
    term("qop", "compiler", "resolver opcode"),
    term("qstk", "compiler", "Forth-inspired bytecode stack tape"),
    term("qpak", "runtime", "budgeted evidence pack"),
    term("qprf", "runtime", "proof page or cited evidence"),
    term("qres", "runtime", "local resolver that expands handles"),
    term("qv0d", "runtime", "resolver miss or unsupported evidence"),
    term("qgat", "safety", "fail-closed assertion gate"),
    term("qcas", "cache", "source-level cache policy"),
    term("qkey", "cache", "deterministic cache key source"),
    term("qttl", "cache", "cache lifetime in seconds"),
    term("qidx", "data", "local repo index"),
    term("qvec", "data", "hashed sparse vector"),
    term("qsym", "data", "indexed symbol"),
    term("qsig", "data", "structural signal mask"),
    term("qmap", "runtime", "path and symbol relation map"),
    term("qsqz", "runtime", "query-focused squeeze extract"),
    term("qans", "runtime", "strict evidence answer"),
    term("qask", "language", "bound user question"),
    term("qrun", "runtime", "program execution"),
    term("qexe", "runtime", "local executable runtime"),
    term("qvm", "runtime", "context virtual machine contract"),
    term("qfx", "runtime", "context fault carrier"),
    term("qref", "evidence", "evidence reference"),
    term("qcit", "evidence", "citation-bearing excerpt"),
    term("qatt", "security", "attestation report"),
    term("qsgn", "security", "signature material"),
    term("qhot", "cache", "hot local cache state"),
    term("qkv", "adapter", "KV hint manifest"),
    term("qlat", "data", "local lattice state"),
    term("qmem", "data", "local memory item"),
    term("qfed", "data", "local file federation bundle"),
    term("qcap", "handle", "capsule handle"),
    term("qses", "handle", "session handle"),
    term("qsng", "handle", "single compact qorx:// handle"),
    term("qevt", "handle", "event receipt handle"),
    term("qrc", "cache", "runtime cache key prefix"),
    term("qpb", "data", "protobuf envelope"),
    term("qapi", "surface", "API surface"),
    term("qa2a", "surface", "A2A task surface"),
    term("qmcp", "surface", "MCP bridge surface"),
    term("qcli", "surface", "command line surface"),
    term("qfmt", "tooling", "future formatter"),
    term("qlnt", "tooling", "future linter"),
    term("qmod", "language", "future module unit"),
    term("qfn", "language", "future function unit"),
    term("qimp", "language", "future import unit"),
    term("qif", "language", "future conditional branch"),
    term("qels", "language", "future else branch"),
    term("qstd", "language", "future standard library"),
    term("qffi", "adapter", "external runtime adapter"),
    term("qusd", "accounting", "estimated USD accounting"),
    term("qbud", "accounting", "token budget"),
    term("qalc", "accounting", "budgeted quark allocator"),
    term("qfld", "data", "active resolution field"),
    term("qspn", "data", "quark priority orientation"),
    term("qgrv", "data", "relevance weight"),
    term("qflx", "data", "context mass delta"),
    term("qfus", "runtime", "merge packs or capsules"),
    term("qrng", "evidence", "line or byte range"),
    term("qdag", "compiler", "dependency graph"),
];

const fn term(name: &'static str, kind: &'static str, meaning: &'static str) -> LexiconTerm {
    LexiconTerm {
        name,
        kind,
        meaning,
    }
}

pub fn report() -> LexiconReport {
    LexiconReport {
        schema: "qorx.lexicon.v1".to_string(),
        language: "qorx".to_string(),
        format: "protobuf-envelope".to_string(),
        vocabulary: vocabulary(),
        aliases: aliases(),
        terms: TERMS,
        layers: json!({
            "ai_language": "Qorx is an AI language and local context runtime.",
            "encoding": "Qorx state and bytecode use protobuf-envelope storage.",
            "physics_terms": "Primary Qorx terms stay 3-4 characters. Long physics words are only legacy aliases and not physics claims.",
            "phot": "tiny model-visible carrier: prompt block, A2A message, or qorx:// handle",
            "qwav": ".qorx source before execution",
            "qfal": ".qorx parsed into deterministic opcodes or .qorxb bytecode",
            "qstk": "Forth-inspired stack tape inside .qorxb for tiny local dispatch",
            "qosm": "local protobuf ledger and data directory where conversations, capsules, and action receipts are retained",
            "qres": "local Qorx runtime that expands tiny handles into exact indexed evidence",
            "qsng": "compact qorx:// handle that points back to local Qorx state",
            "hzon": "explicit boundary between local evidence and provider-visible tokens",
            "mass": "deterministic local token estimate, not provider tokenizer billing truth",
            "qshf": "baseline-to-compact accounting ratio",
            "cost_transform": "measured qshf compaction backed by local counters"
        }),
        boundary: "Qorx uses quark-inspired vocabulary, but it is not a physics engine and does not claim physical compression. provider billing is not bypassed; outside cost depends on the actual upstream request sent.".to_string(),
    }
}

pub fn vocabulary() -> Value {
    json!({
        ".qorx": "qwav_source",
        ".qorxb": "qfal_bytecode",
        "qorx://s": "qses_handle",
        "qorx://c": "qcap_handle",
        "qorx://l": "qlat_handle",
        "qorx://f": "qfed_handle",
        "qorx://u": "qsng_handle",
        "qstk": "forth_like_stack_tape",
        "qosm": "local_resolver_ledger",
        "qshf": "baseline_to_compact_accounting",
        "qv0d": "resolver_miss_or_empty_evidence",
        "cosmos_store": "qorx_data_dir_or_portable_store",
        "qorx_data_dir": "qosm_storage",
        "qorx-cosmos.pb": "qpb_qosm_ledger",
        "prompt_block": "phot_carrier",
        "visible_tokens": "phot_mass",
        "indexed_tokens": "qosm_mass",
        "b2c": "qshf_accounting",
        "cost_transform": "qshf_compaction_transform",
        "context_reduction_x": "qshf_factor",
        "provider_calls": "external_observation_count"
    })
}

pub fn aliases() -> Value {
    json!({
        "quark": "qrk",
        "cosmos": "qosm",
        "wavefunction": "qwav",
        "collapse": "qfal",
        "photon": "phot",
        "event_horizon": "hzon",
        "redshift": "qshf",
        "rshift": "qshf",
        "qshift": "qshf",
        "qvoid": "qv0d",
        "void": "qv0d",
        "proof": "qprf",
        "capsule": "qcap",
        "session": "qses"
    })
}

pub fn runtime_tags(source_kind: &str) -> Value {
    json!({
        "source": if source_kind == "qorxb" { "qfal" } else { "qwav" },
        "model_visible_carrier": "phot",
        "local_runtime": "qosm",
        "storage": "q_drive_or_qorx_data_dir",
        "handle": "qsng",
        "cost_transform": "qshf",
        "boundary": "hzon"
    })
}
