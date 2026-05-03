use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::cost_stack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterReport {
    pub philosophy: String,
    pub adapters: Vec<AdapterStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScienceReport {
    pub claim_boundary: String,
    pub built_in_logic: Vec<ScienceFeature>,
    pub applied_adapter_science: Vec<ScienceFeature>,
    pub external_runtime_adapters: Vec<AdapterStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScienceFeature {
    pub name: String,
    pub active: bool,
    pub proof: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatus {
    pub name: String,
    pub kind: String,
    pub configured: bool,
    pub ready: bool,
    pub command_or_url: Option<String>,
    pub reason: String,
}

pub fn adapter_report() -> AdapterReport {
    AdapterReport {
        philosophy: "manifest-first, payload-later: keep fingerprints, symbols, sparse vectors, cache keys, and benchmark proof in RAM; load exact payload quarks only on demand".to_string(),
        adapters: vec![
            command_adapter(
                "tree-sitter parser packs",
                "parser",
                &["QORX_TREESITTER_CMD"],
                &["tree-sitter"],
            ),
            command_adapter(
                "LLMLingua compressor",
                "prompt_compressor",
                &["QORX_LLMLINGUA_CMD"],
                &["llmlingua", "llmlingua2"],
            ),
            command_adapter(
                "ONNX compressor",
                "prompt_compressor",
                &["QORX_ONNX_COMPRESSOR_CMD"],
                &["onnxruntime", "ort"],
            ),
            command_adapter(
                "RotorQuant KV backend",
                "kv_cache_runtime",
                &["QORX_ROTORQUANT_CMD"],
                &["rotorquant"],
            ),
            command_adapter(
                "IsoQuant KV backend",
                "kv_cache_runtime",
                &["QORX_ISOQUANT_CMD"],
                &["isoquant"],
            ),
            command_adapter(
                "TurboQuant/vLLM KV backend",
                "kv_cache_runtime",
                &["QORX_TURBOQUANT_CMD", "QORX_KV_BACKEND"],
                &["turboquant", "vllm"],
            ),
            url_or_command_adapter(
                "embedding/vector backend",
                "embedding",
                &["QORX_EMBEDDING_URL"],
                &["QORX_EMBEDDING_CMD"],
                &["fastembed", "tei", "ollama"],
            ),
            paper_qa_adapter(),
        ],
    }
}

pub fn science_report() -> ScienceReport {
    ScienceReport {
        claim_boundary: "Qorx implements local qshf/B2C accounting and evidence selection. External ML/KV runtimes are adapter targets and must be installed separately before Qorx can call them.".to_string(),
        built_in_logic: vec![
            feature(
                "manifest-first local memory",
                "repo chunks are indexed as small quarks; full payloads are loaded only when a query asks for them",
            ),
            feature(
                "sparse vector DB",
                "each quark stores hashed sparse term vectors inside repo_index.pb; no neural model required",
            ),
            feature(
                "protobuf state store",
                "Qorx-owned state files are persisted as .pb files, and legacy .json files are read only as migration inputs",
            ),
            feature(
                "structural code signals",
                "indexing records symbols plus import/route/test/error/branch/assignment/call/config signal masks",
            ),
            feature(
                "token-budgeted packer",
                "qorx.pack returns only ranked evidence that fits the requested token budget",
            ),
            feature(
                "diff-aware impact packer",
                "qorx.impact parses unified git diffs, follows local symbol edges, and returns changed plus related quarks under budget",
            ),
            feature(
                "benchmark proof",
                "qorx.bench reports indexed tokens, sent tokens, omitted tokens, quarks used, and reduction factor",
            ),
            feature(
                "session pointer mode",
                "qorx.session emits a tiny qorx://s handle that keeps repo memory local and requires exact retrieval through Qorx tools",
            ),
            feature(
                "portable local store",
                "qorx.portable keeps index, quarks, cache, stats, and shims beside qorx.exe when qorx.portable or QORX_PORTABLE is active",
            ),
            feature(
                "exact replay cache",
                "CE stores exact replay cache data structures for review; managed routed-provider cache behavior belongs to Qorx Local Pro or Qorx API",
            ),
            feature(
                "local USD estimator",
                "stats converts omitted local input tokens into estimated USD with QORX_USD_PER_M_INPUT_TOKENS overrides; provider invoice claims need routed billing evidence outside CE",
            ),
        ],
        applied_adapter_science: cost_stack::APPLIED_SCIENCE
            .iter()
            .map(|science| feature(science.name, science.proof))
            .collect(),
        external_runtime_adapters: adapter_report().adapters,
    }
}

fn feature(name: &str, proof: &str) -> ScienceFeature {
    ScienceFeature {
        name: name.to_string(),
        active: true,
        proof: proof.to_string(),
    }
}

fn command_adapter(
    name: &str,
    kind: &str,
    env_names: &[&str],
    path_candidates: &[&str],
) -> AdapterStatus {
    for env_name in env_names {
        if let Ok(value) = env::var(env_name) {
            return AdapterStatus {
                name: name.to_string(),
                kind: kind.to_string(),
                configured: true,
                ready: true,
                command_or_url: Some(value),
                reason: format!("configured through {env_name}"),
            };
        }
    }

    for candidate in path_candidates {
        if let Some(path) = resolve_on_path(candidate) {
            return AdapterStatus {
                name: name.to_string(),
                kind: kind.to_string(),
                configured: false,
                ready: true,
                command_or_url: Some(path.display().to_string()),
                reason: format!("found {candidate} on PATH"),
            };
        }
    }

    AdapterStatus {
        name: name.to_string(),
        kind: kind.to_string(),
        configured: false,
        ready: false,
        command_or_url: None,
        reason: format!("set {} to enable this adapter", env_names.join(" or ")),
    }
}

fn url_or_command_adapter(
    name: &str,
    kind: &str,
    url_env_names: &[&str],
    command_env_names: &[&str],
    path_candidates: &[&str],
) -> AdapterStatus {
    for env_name in url_env_names {
        if let Ok(value) = env::var(env_name) {
            return AdapterStatus {
                name: name.to_string(),
                kind: kind.to_string(),
                configured: true,
                ready: true,
                command_or_url: Some(value),
                reason: format!("configured through {env_name}"),
            };
        }
    }
    command_adapter(name, kind, command_env_names, path_candidates)
}

fn paper_qa_adapter() -> AdapterStatus {
    let backend_configured = any_env_set(&[
        "PAPERQA_LLM",
        "PAPERQA_EMBEDDING",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GOOGLE_API_KEY",
        "GEMINI_API_KEY",
        "XAI_API_KEY",
    ]);
    let Ok(root) = env::current_dir() else {
        return AdapterStatus {
            name: "PaperQA research corpus".to_string(),
            kind: "research_qa".to_string(),
            configured: backend_configured,
            ready: false,
            command_or_url: None,
            reason: "could not resolve current repo root for PaperQA probe".to_string(),
        };
    };
    paper_qa_adapter_from_root(&root, backend_configured)
}

fn paper_qa_adapter_from_root(root: &Path, backend_configured: bool) -> AdapterStatus {
    let command = find_command_near_root(root, &["pqa", "paper-qa"])
        .or_else(|| resolve_on_path("pqa"))
        .or_else(|| resolve_on_path("paper-qa"));
    let has_corpus = has_paper_corpus(root);
    let command_text = command.as_ref().map(|path| path.display().to_string());
    let ready = command.is_some() && has_corpus && backend_configured;
    let reason = match (command.is_some(), has_corpus, backend_configured) {
        (true, true, true) => {
            "configured: pqa command, local paper corpus, and LLM/embedding backend are available"
                .to_string()
        }
        (true, true, false) => {
            "pqa command and local paper corpus found, but no LLM or embedding backend env is configured"
                .to_string()
        }
        (true, false, _) => {
            "pqa command found, but research/papers has no local PDF corpus".to_string()
        }
        (false, true, _) => {
            "local paper corpus found, but pqa command was not found".to_string()
        }
        (false, false, _) => {
            "set up .venv with pqa and research/papers PDFs to enable PaperQA".to_string()
        }
    };

    AdapterStatus {
        name: "PaperQA research corpus".to_string(),
        kind: "research_qa".to_string(),
        configured: ready,
        ready,
        command_or_url: command_text,
        reason,
    }
}

fn has_paper_corpus(root: &Path) -> bool {
    let papers = root.join("research").join("papers");
    let Ok(entries) = std::fs::read_dir(papers) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false)
    })
}

fn any_env_set(names: &[&str]) -> bool {
    names.iter().any(|name| {
        env::var(name)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

fn resolve_on_path(name: &str) -> Option<PathBuf> {
    find_command_in_dirs(name, search_dirs())
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = env::var_os("PATH")
        .map(|path_env| env::split_paths(&path_env).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Ok(root) = env::current_dir() {
        dirs.extend(local_tool_dirs(&root));
    }
    dirs
}

fn find_command_near_root(root: &Path, names: &[&str]) -> Option<PathBuf> {
    let dirs = local_tool_dirs(root);
    for name in names {
        if let Some(path) = find_command_in_dirs(name, dirs.clone()) {
            return Some(path);
        }
    }
    None
}

fn local_tool_dirs(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join(".venv").join("Scripts"),
        root.join(".venv").join("bin"),
        root.join("node_modules").join(".bin"),
    ]
}

fn find_command_in_dirs(name: &str, dirs: Vec<PathBuf>) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![
            OsString::from(format!("{name}.exe")),
            OsString::from(format!("{name}.cmd")),
            OsString::from(format!("{name}.bat")),
            OsString::from(format!("{name}.ps1")),
            OsString::from(name),
        ]
    } else {
        vec![OsString::from(name)]
    };

    for dir in dirs {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if is_file(&full) {
                return Some(full);
            }
        }
    }
    None
}

fn is_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("{prefix}-{suffix}-{sequence}"))
    }

    fn seed_paperqa(root: &Path) {
        let scripts = root.join(".venv").join("Scripts");
        fs::create_dir_all(&scripts).expect("create scripts");
        fs::write(scripts.join("pqa.exe"), "stub").expect("write pqa");
        let papers = root.join("research").join("papers");
        fs::create_dir_all(&papers).expect("create papers");
        fs::write(papers.join("llmlingua.pdf"), "stub").expect("write paper");
    }

    #[test]
    fn paper_qa_reports_local_corpus_but_blocks_without_model_backend() {
        let root = unique_temp_dir("qorx-paperqa-adapter");
        seed_paperqa(&root);

        let status = super::paper_qa_adapter_from_root(&root, false);

        assert_eq!(status.name, "PaperQA research corpus");
        assert_eq!(status.kind, "research_qa");
        assert!(!status.configured);
        assert!(!status.ready);
        assert!(status.command_or_url.unwrap().ends_with("pqa.exe"));
        assert!(status.reason.contains("no LLM or embedding backend"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn paper_qa_is_ready_when_local_corpus_and_backend_are_configured() {
        let root = unique_temp_dir("qorx-paperqa-adapter");
        seed_paperqa(&root);

        let status = super::paper_qa_adapter_from_root(&root, true);

        assert!(status.configured);
        assert!(status.ready);
        assert!(status.reason.contains("configured"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn science_report_exposes_applied_adapter_science_without_runtime_kv_claims() {
        let report = super::science_report();

        let names = report
            .applied_adapter_science
            .iter()
            .map(|feature| feature.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"LLMLingua prompt compression principle"));
        assert!(names.contains(&"TurboQuant/vLLM KV boundary"));

        let kv_boundary = report
            .applied_adapter_science
            .iter()
            .find(|feature| feature.name == "TurboQuant/vLLM KV boundary")
            .expect("TurboQuant/vLLM boundary should be listed");
        assert!(kv_boundary.active);
        assert!(kv_boundary
            .proof
            .contains("safetensors-compatible KV hints"));
        assert!(kv_boundary
            .proof
            .contains("not runtime KV tensor compression"));
    }
}
