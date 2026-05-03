use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    aim,
    compression::{estimate_tokens, TOKEN_ESTIMATOR_LABEL},
    config::AppPaths,
    cost_stack,
    index::{self, IndexOptions, RepoAtom, RepoIndex},
    memory,
    truth::{self, StrictAnswer},
};

const CAPSULE_FILE: &str = "qorx-capsule.pb";
const CAPSULE_SESSION_FILE: &str = "qorx-capsule-session.pb";

#[derive(Debug, Clone)]
pub struct CapsuleCreateOptions {
    pub include_memory: bool,
    pub include_aim: bool,
    pub include_sensitive: bool,
    pub max_files: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrvinCandidate {
    pub kind: String,
    pub path: String,
    pub selected: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrvinReport {
    pub schema: String,
    pub loaded: bool,
    pub message: String,
    pub candidates: Vec<BrvinCandidate>,
    pub capsule: Capsule,
    pub next: Vec<String>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub schema: String,
    pub handle: String,
    pub created_at: String,
    pub root: String,
    pub root_kind: String,
    pub sources: Vec<CapsuleSource>,
    pub quark_count: usize,
    pub indexed_tokens: u64,
    pub visible_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    pub index_sha256: String,
    pub prompt_block: String,
    pub index: RepoIndex,
    pub boundary: String,
    pub safety: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleSessionPointer {
    pub schema: String,
    pub handle: String,
    pub created_at: String,
    pub source_count: usize,
    pub quark_count: usize,
    pub indexed_tokens: u64,
    pub visible_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    pub prompt_block: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleSource {
    pub kind: String,
    pub path: Option<String>,
    pub included: bool,
    pub quarks: usize,
    pub indexed_tokens: u64,
    pub bytes: u64,
    pub fingerprint: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleStrictReport {
    pub schema: String,
    pub capsule: String,
    pub answer: StrictAnswer,
    pub boundary: String,
}

struct PromptParts<'a> {
    handle: &'a str,
    source_count: usize,
    quark_count: usize,
    indexed_tokens: u64,
    visible_tokens: u64,
    omitted_tokens: u64,
    context_reduction_x: f64,
    created_at: &'a str,
}

pub fn capsule_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(CAPSULE_FILE)
}

pub fn capsule_session_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(CAPSULE_SESSION_FILE)
}

pub fn create(paths: &AppPaths, root: &Path, options: CapsuleCreateOptions) -> Result<Capsule> {
    let index_options = IndexOptions {
        lenient: true,
        max_files: options.max_files,
        include_sensitive: options.include_sensitive,
    };
    let mut index = index::build_index_value(root, &index_options)?;
    let mut sources = vec![CapsuleSource {
        kind: "folder".to_string(),
        path: Some(index.root.clone()),
        included: true,
        quarks: index.atoms.len(),
        indexed_tokens: index.total_tokens(),
        bytes: 0,
        fingerprint: Some(index_fingerprint(&index)),
        note: "Local folder/project/brain root indexed into hashed quarks.".to_string(),
    }];

    if options.include_memory {
        let before = index.atoms.len();
        append_memory_atoms(paths, &mut index)?;
        let added = &index.atoms[before..];
        sources.push(CapsuleSource {
            kind: "marvin_memory".to_string(),
            path: Some(memory::memory_path(paths).display().to_string()),
            included: !added.is_empty(),
            quarks: added.len(),
            indexed_tokens: added.iter().map(|atom| atom.token_estimate).sum(),
            bytes: 0,
            fingerprint: Some(hash_atoms(added)),
            note: "Local Marvin memory cards included as explicit capsule evidence.".to_string(),
        });
    }

    if options.include_aim {
        let before = index.atoms.len();
        let aim_report = aim::inspect_default()?;
        append_aim_atom(&aim_report, &mut index);
        let added = &index.atoms[before..];
        sources.push(CapsuleSource {
            kind: "cosmos_sidecar".to_string(),
            path: aim_report.path.clone(),
            included: !added.is_empty(),
            quarks: added.len(),
            indexed_tokens: added.iter().map(|atom| atom.token_estimate).sum(),
            bytes: aim_report.bytes,
            fingerprint: aim_report.fingerprint,
            note: "Optional local sidecar metadata included; binary payload stays local inside the Qorx cosmos boundary.".to_string(),
        });
    }

    index.updated_at = Utc::now();
    index.atoms.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.id.cmp(&b.id))
    });
    let capsule = build_capsule(index, sources);
    crate::proto_store::save(&capsule_path(paths), &capsule)?;
    save_session_pointer(paths, &capsule)?;
    Ok(capsule)
}

pub fn create_auto(paths: &AppPaths, options: CapsuleCreateOptions) -> Result<BrvinReport> {
    let candidates = detect_brvin_candidates();
    let selected = candidates
        .iter()
        .filter(|candidate| candidate.selected)
        .map(|candidate| PathBuf::from(&candidate.path))
        .collect::<Vec<_>>();
    let roots = if selected.is_empty() {
        vec![env::current_dir()?]
    } else {
        selected
    };
    let capsule = create_from_roots(paths, &roots, &options)?;
    Ok(BrvinReport {
        schema: "qorx.cosmos-capsule.v1".to_string(),
        loaded: true,
        message: "Qorx cosmos capsule is loaded".to_string(),
        candidates,
        capsule,
        next: vec![
            "Use qorx capsule session --block to copy the tiny capsule prompt.".to_string(),
            "Use qorx capsule create <folder> --include-memory for a manual project, repo, RAG brain, notes vault, or exported database folder.".to_string(),
            "Use qorx capsule strict-answer <question> to answer only from capsule evidence.".to_string(),
        ],
        boundary: "The Qorx cosmos resolver auto-detects likely local project and memory roots and builds a local evidence capsule. Private file upload, literal physical compression, and downstream model correctness are outside this local report.".to_string(),
    })
}

pub fn detect_brvin_candidates() -> Vec<BrvinCandidate> {
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    if let Ok(current) = env::current_dir() {
        if let Some(git_root) = find_git_root(&current) {
            push_candidate(
                &mut candidates,
                &mut seen,
                "git_project",
                git_root,
                true,
                "Current git repository.",
            );
        } else {
            push_candidate(
                &mut candidates,
                &mut seen,
                "current_folder",
                current,
                true,
                "Current working folder.",
            );
        }
    }
    for key in ["QORX_BRVIN", "QORX_BRAIN", "BRVIN_HOME"] {
        if let Some(path) = env::var_os(key).map(PathBuf::from) {
            push_candidate(
                &mut candidates,
                &mut seen,
                "env_brvin",
                path,
                true,
                "Explicit brain-vault environment root.",
            );
        }
    }
    if let Some(home) = home_dir() {
        for (kind, path, selected, note) in [
            (
                "documents_brain",
                home.join("Documents").join("brain"),
                true,
                "Default Marvin brain/RAG folder.",
            ),
            (
                "codex_home",
                home.join(".codex"),
                true,
                "Codex CLI home: memories, hooks, skills, rules, sessions, and local config.",
            ),
            (
                "codex_memories",
                home.join(".codex").join("memories"),
                true,
                "Codex local memory registry; kept for compatibility when full Codex home is absent.",
            ),
            (
                "gemini_cli_home",
                home.join(".gemini"),
                true,
                "Gemini CLI user memory/config folder.",
            ),
            (
                "antigravity_home",
                home.join(".antigravity"),
                true,
                "Antigravity user home and extensions.",
            ),
            (
                "antigravity_cockpit_home",
                home.join(".antigravity_cockpit"),
                true,
                "Antigravity Cockpit user home.",
            ),
            (
                "opencode_home",
                home.join(".opencode"),
                true,
                "OpenCode user home when installed.",
            ),
            (
                "kilocode_home",
                home.join(".kilocode"),
                true,
                "KiloCode user home and skills.",
            ),
            (
                "openclaw_home",
                home.join(".openclaw"),
                true,
                "OpenClaw user home when installed.",
            ),
            (
                "openhands_home",
                home.join(".openhands"),
                true,
                "OpenHands user home when installed.",
            ),
            (
                "claude_memory",
                home.join(".claude"),
                true,
                "Claude Code user memory/config folder.",
            ),
            (
                "obsidian_vaults",
                home.join("Documents").join("Obsidian"),
                true,
                "Common Obsidian vault parent.",
            ),
            (
                "notes_vaults",
                home.join("Documents").join("Notes"),
                true,
                "Common notes/RAG vault parent.",
            ),
        ] {
            push_candidate(&mut candidates, &mut seen, kind, path, selected, note);
        }
    }
    for (kind, path, selected, note) in [
        (
            "alpine_wsl_root",
            PathBuf::from(r"\\wsl$\Alpine"),
            true,
            "Alpine WSL distro root; pseudo/system caches are skipped by the indexer.",
        ),
        (
            "alpine_wsl_home",
            PathBuf::from(r"\\wsl$\Alpine\home\marvin"),
            true,
            "Alpine WSL user home.",
        ),
    ] {
        push_candidate(&mut candidates, &mut seen, kind, path, selected, note);
    }
    if let Some(aim_path) =
        aim::resolve_aim_path().and_then(|path| path.parent().map(Path::to_path_buf))
    {
        push_candidate(
            &mut candidates,
            &mut seen,
            "aim_parent",
            aim_path,
            true,
            "Detected optional local sidecar parent.",
        );
    }
    candidates
}

pub fn load(paths: &AppPaths) -> Result<Capsule> {
    let path = capsule_path(paths);
    let legacy = path.with_extension("json");
    crate::proto_store::load_required(&path, &[legacy.as_path()])
}

pub fn load_session_pointer(paths: &AppPaths) -> Result<CapsuleSessionPointer> {
    let path = capsule_session_path(paths);
    if let Ok(mut pointer) = crate::proto_store::load_required::<CapsuleSessionPointer>(&path, &[])
    {
        let upgraded = ensure_cost_stack_tag(&pointer.prompt_block);
        if upgraded != pointer.prompt_block {
            pointer.prompt_block = upgraded;
            crate::proto_store::save(&path, &pointer)?;
        }
        return Ok(pointer);
    }
    let capsule = load(paths)?;
    let pointer = session_pointer(&capsule);
    crate::proto_store::save(&path, &pointer)?;
    Ok(pointer)
}

pub fn save_session_pointer(paths: &AppPaths, capsule: &Capsule) -> Result<()> {
    crate::proto_store::save(&capsule_session_path(paths), &session_pointer(capsule))
}

pub fn strict_answer(
    paths: &AppPaths,
    question: &str,
    limit: usize,
) -> Result<CapsuleStrictReport> {
    let capsule = load(paths)?;
    let answer = truth::strict_answer(&capsule.index, question, limit);
    Ok(CapsuleStrictReport {
        schema: "qorx.capsule.strict-answer.v1".to_string(),
        capsule: capsule.handle,
        answer,
        boundary: "Capsule strict-answer is evidence-only: it answers from the saved local capsule index or returns not_found.".to_string(),
    })
}

fn create_from_roots(
    paths: &AppPaths,
    roots: &[PathBuf],
    options: &CapsuleCreateOptions,
) -> Result<Capsule> {
    let index_options = IndexOptions {
        lenient: true,
        max_files: options.max_files,
        include_sensitive: options.include_sensitive,
    };
    let mut merged = RepoIndex {
        root: "brvin://auto".to_string(),
        updated_at: Utc::now(),
        atoms: Vec::new(),
    };
    let mut sources = Vec::new();
    for root in roots {
        let index = index::build_index_value(root, &index_options)?;
        let prefix = source_prefix(&index.root);
        let before = merged.atoms.len();
        for mut atom in index.atoms {
            atom.path = format!("{prefix}/{}", atom.path);
            merged.atoms.push(atom);
        }
        let added = &merged.atoms[before..];
        sources.push(CapsuleSource {
            kind: "brvin_root".to_string(),
            path: Some(index.root),
            included: !added.is_empty(),
            quarks: added.len(),
            indexed_tokens: added.iter().map(|atom| atom.token_estimate).sum(),
            bytes: 0,
            fingerprint: Some(hash_atoms(added)),
            note: "Auto-detected local brain/project root included in the brain-vault capsule."
                .to_string(),
        });
    }
    if options.include_memory {
        let before = merged.atoms.len();
        append_memory_atoms(paths, &mut merged)?;
        let added = &merged.atoms[before..];
        sources.push(CapsuleSource {
            kind: "marvin_memory".to_string(),
            path: Some(memory::memory_path(paths).display().to_string()),
            included: !added.is_empty(),
            quarks: added.len(),
            indexed_tokens: added.iter().map(|atom| atom.token_estimate).sum(),
            bytes: 0,
            fingerprint: Some(hash_atoms(added)),
            note: "Local Marvin memory cards included as explicit capsule evidence.".to_string(),
        });
    }
    if options.include_aim {
        let before = merged.atoms.len();
        let aim_report = aim::inspect_default()?;
        append_aim_atom(&aim_report, &mut merged);
        let added = &merged.atoms[before..];
        sources.push(CapsuleSource {
            kind: "cosmos_sidecar".to_string(),
            path: aim_report.path.clone(),
            included: !added.is_empty(),
            quarks: added.len(),
            indexed_tokens: added.iter().map(|atom| atom.token_estimate).sum(),
            bytes: aim_report.bytes,
            fingerprint: aim_report.fingerprint,
            note: "Optional local sidecar metadata included; binary payload stays local inside the Qorx cosmos boundary.".to_string(),
        });
    }
    merged.updated_at = Utc::now();
    merged.atoms.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.id.cmp(&b.id))
    });
    let capsule = build_capsule(merged, sources);
    crate::proto_store::save(&capsule_path(paths), &capsule)?;
    save_session_pointer(paths, &capsule)?;
    Ok(capsule)
}

fn session_pointer(capsule: &Capsule) -> CapsuleSessionPointer {
    CapsuleSessionPointer {
        schema: "qorx.capsule-session.v1".to_string(),
        handle: capsule.handle.clone(),
        created_at: capsule.created_at.clone(),
        source_count: capsule.sources.len(),
        quark_count: capsule.quark_count,
        indexed_tokens: capsule.indexed_tokens,
        visible_tokens: capsule.visible_tokens,
        omitted_tokens: capsule.omitted_tokens,
        context_reduction_x: capsule.context_reduction_x,
        prompt_block: ensure_cost_stack_tag(&capsule.prompt_block),
    }
}

fn ensure_cost_stack_tag(prompt_block: &str) -> String {
    if prompt_block.contains(cost_stack::PROMPT_TAG) {
        return prompt_block.to_string();
    }
    let normalized = prompt_block
        .replace("cosmos=core", "qosm=core")
        .replace("redshift=core_b2c", "qshf=core_b2c")
        .replace("qshift=core_b2c", "qshf=core_b2c");
    if normalized.contains(cost_stack::PROMPT_TAG) {
        return normalized;
    }
    let trimmed = normalized.trim_end();
    if trimmed.is_empty() {
        cost_stack::PROMPT_TAG.to_string()
    } else {
        format!("{trimmed} {}", cost_stack::PROMPT_TAG)
    }
}

fn build_capsule(index: RepoIndex, sources: Vec<CapsuleSource>) -> Capsule {
    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let index_sha256 = index_fingerprint(&index);
    let handle = format!("qorx://c/{}", &index_sha256[..16]);
    let indexed_tokens = index.total_tokens();
    let mut visible_tokens = 1;
    let mut omitted_tokens = indexed_tokens;
    let mut context_reduction_x = indexed_tokens.max(1) as f64;
    let mut prompt_block = String::new();
    for _ in 0..4 {
        prompt_block = build_prompt_block(PromptParts {
            handle: &handle,
            source_count: sources.len(),
            quark_count: index.atoms.len(),
            indexed_tokens,
            visible_tokens,
            omitted_tokens,
            context_reduction_x,
            created_at: &created_at,
        });
        let next_visible_tokens = estimate_tokens(&prompt_block).max(1);
        let next_omitted_tokens = indexed_tokens.saturating_sub(next_visible_tokens);
        let next_context_reduction_x =
            indexed_tokens.max(1) as f64 / next_visible_tokens.max(1) as f64;
        if next_visible_tokens == visible_tokens {
            break;
        }
        visible_tokens = next_visible_tokens;
        omitted_tokens = next_omitted_tokens;
        context_reduction_x = next_context_reduction_x;
    }
    let visible_tokens = estimate_tokens(&prompt_block).max(1);
    let omitted_tokens = indexed_tokens.saturating_sub(visible_tokens);
    let context_reduction_x = indexed_tokens.max(1) as f64 / visible_tokens.max(1) as f64;

    Capsule {
        schema: "qorx.capsule.v1".to_string(),
        handle,
        created_at,
        root: index.root.clone(),
        root_kind: "folder_or_brain_or_project".to_string(),
        sources,
        quark_count: index.atoms.len(),
        indexed_tokens,
        visible_tokens,
        omitted_tokens,
        context_reduction_x,
        index_sha256,
        prompt_block,
        index,
        boundary: "A Qorx capsule is a tiny model-visible pointer to local protobuf-envelope evidence. Exact data remains in the local resolver.".to_string(),
        safety: "By default capsule indexing skips common secret/key credential paths and only includes textual evidence files. Use include-sensitive only for controlled private experiments.".to_string(),
    }
}

fn build_prompt_block(parts: PromptParts<'_>) -> String {
    format!(
        "QORX_CAPSULE {handle}\nsources={source_count} q={quark_count} local_idx={indexed_tokens}\nmode=strict-evidence local_pb; local_idx stays local; resolve with Qorx capsule.\nproof at={created_at} ctx={indexed_tokens}t vis={visible_tokens}t saved={omitted_tokens}t qshf={context_reduction_x:.2}x est={TOKEN_ESTIMATOR_LABEL} b2c=accounting {stack}",
        handle = parts.handle,
        source_count = parts.source_count,
        quark_count = parts.quark_count,
        indexed_tokens = parts.indexed_tokens,
        visible_tokens = parts.visible_tokens,
        omitted_tokens = parts.omitted_tokens,
        context_reduction_x = parts.context_reduction_x,
        created_at = parts.created_at,
        stack = cost_stack::PROMPT_TAG,
    )
}

fn append_memory_atoms(paths: &AppPaths, index: &mut RepoIndex) -> Result<()> {
    for item in memory::read_all(paths)? {
        let content_hash =
            hex_sha256(format!("{}\n{}\n{}", item.kind, item.summary, item.text).as_bytes());
        let text = format!(
            "kind: {}\nsummary: {}\ntext: {}",
            item.kind, item.summary, item.text
        );
        index.atoms.push(synthetic_atom(
            "memory",
            &format!("qorx-memory/{}.md", &content_hash[..12]),
            &text,
            item.token_estimate,
        ));
    }
    Ok(())
}

fn append_aim_atom(report: &aim::AimReport, index: &mut RepoIndex) {
    if !report.found {
        return;
    }
    let text = serde_json::to_string_pretty(report).unwrap_or_else(|_| {
        format!(
            "local sidecar path={:?} bytes={} fingerprint={:?}",
            report.path, report.bytes, report.fingerprint
        )
    });
    index.atoms.push(synthetic_atom(
        "sidecar",
        "qorx-sidecar/metadata.json",
        &text,
        estimate_tokens(&text),
    ));
}

fn synthetic_atom(kind: &str, path: &str, text: &str, token_estimate: u64) -> RepoAtom {
    let hash = hex_sha256(format!("{kind}\n{path}\n{text}").as_bytes());
    RepoAtom {
        id: format!("qvc_{}", &hash[..12]),
        path: path.to_string(),
        start_line: 1,
        end_line: text.lines().count().max(1),
        hash,
        token_estimate,
        symbols: vec![kind.to_string()],
        signal_mask: 0,
        vector: Vec::new(),
        text: text.to_string(),
    }
}

fn index_fingerprint(index: &RepoIndex) -> String {
    let mut hasher = Sha256::new();
    hasher.update(index.root.as_bytes());
    for atom in &index.atoms {
        hasher.update(atom.id.as_bytes());
        hasher.update(atom.hash.as_bytes());
        hasher.update(atom.path.as_bytes());
        hasher.update(atom.start_line.to_le_bytes());
        hasher.update(atom.end_line.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn hash_atoms(atoms: &[RepoAtom]) -> String {
    let mut hasher = Sha256::new();
    for atom in atoms {
        hasher.update(atom.hash.as_bytes());
        hasher.update(atom.path.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn push_candidate(
    candidates: &mut Vec<BrvinCandidate>,
    seen: &mut BTreeSet<String>,
    kind: &str,
    path: PathBuf,
    selected: bool,
    note: &str,
) {
    if !path.is_dir() {
        return;
    }
    let canonical = path.canonicalize().unwrap_or(path);
    let key = canonical.display().to_string().to_lowercase();
    if !seen.insert(key) {
        return;
    }
    candidates.push(BrvinCandidate {
        kind: kind.to_string(),
        path: canonical.display().to_string(),
        selected,
        note: note.to_string(),
    });
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn source_prefix(root: &str) -> String {
    let hash = hex_sha256(root.as_bytes());
    let name = Path::new(root)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("root")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .take(24)
        .collect::<String>();
    let name = if name.is_empty() {
        "root".to_string()
    } else {
        name
    };
    format!("brvin-{name}-{}", &hash[..8])
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::config::AppPaths;

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("{prefix}-{suffix}-{sequence}"))
    }

    fn test_paths(data_dir: &Path) -> AppPaths {
        AppPaths {
            data_dir: data_dir.to_path_buf(),
            portable: false,
            stats_file: data_dir.join("stats.pb"),
            atom_file: data_dir.join("quarks.pb"),
            index_file: data_dir.join("repo_index.pb"),
            context_protobuf_file: data_dir.join("qorx-context.pb"),
            response_cache_file: data_dir.join("response_cache.pb"),
            integration_report_file: data_dir.join("integrations.pb"),
            provenance_file: data_dir.join("qorx-provenance.pb"),
            security_keys_file: data_dir.join("qorx-security-keys.pb"),
            shim_dir: data_dir.join("shims"),
        }
    }

    #[test]
    fn load_session_pointer_upgrades_legacy_prompt_stack_tag() {
        let root = unique_temp_dir("qorx-capsule-session");
        let paths = test_paths(&root);
        fs::create_dir_all(&paths.data_dir).expect("create data dir");
        let stale_pointer = super::CapsuleSessionPointer {
            schema: "qorx.capsule-session.v1".to_string(),
            handle: "qorx://c/stale".to_string(),
            created_at: "2026-04-30T00:00:00Z".to_string(),
            source_count: 1,
            quark_count: 2,
            indexed_tokens: 10_000,
            visible_tokens: 60,
            omitted_tokens: 9_940,
            context_reduction_x: 166.0,
            prompt_block: "QORX_CAPSULE qorx://c/stale\nproof at=2026-04-30T00:00:00Z ctx=10000t vis=60t saved=9940t red=166.00x est=char4 b2c=accounting".to_string(),
        };
        crate::proto_store::save(&super::capsule_session_path(&paths), &stale_pointer)
            .expect("save stale pointer");

        let loaded = super::load_session_pointer(&paths).expect("load upgraded pointer");

        assert!(loaded.prompt_block.contains("qosm=core"));
        assert!(loaded.prompt_block.contains("qshf=core_b2c"));
        assert!(!loaded.prompt_block.contains("redshift=core_b2c"));
        let persisted: super::CapsuleSessionPointer =
            crate::proto_store::load_required(&super::capsule_session_path(&paths), &[])
                .expect("load persisted pointer");
        assert!(persisted.prompt_block.contains("qosm=core"));
        assert!(persisted.prompt_block.contains("qshf=core_b2c"));
        assert!(!persisted.prompt_block.contains("redshift=core_b2c"));

        let _ = fs::remove_dir_all(root);
    }
}
