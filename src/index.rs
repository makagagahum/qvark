use std::{collections::BTreeMap, fs, io::Read, path::Path};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compression::estimate_tokens;

const MAX_FILE_BYTES: u64 = 512 * 1024;
const MAX_EPUB_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EPUB_ENTRY_BYTES: u64 = 2 * 1024 * 1024;
const MAX_EPUB_TEXT_ENTRIES: usize = 512;
const MAX_CHUNK_LINES: usize = 80;
const MAX_CHUNK_CHARS: usize = 4_000;
const MAX_VECTOR_TERMS: usize = 96;
const SIG_IMPORT: u16 = 1 << 0;
const SIG_ROUTE: u16 = 1 << 1;
const SIG_TEST: u16 = 1 << 2;
const SIG_ERROR: u16 = 1 << 3;
const SIG_BRANCH: u16 = 1 << 4;
const SIG_ASSIGNMENT: u16 = 1 << 5;
const SIG_CALL: u16 = 1 << 6;
const SIG_CONFIG: u16 = 1 << 7;

#[derive(Debug, Clone, Default)]
pub struct IndexOptions {
    pub lenient: bool,
    pub max_files: Option<usize>,
    pub include_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIndex {
    pub root: String,
    pub updated_at: DateTime<Utc>,
    #[serde(default, rename = "quarks", alias = "atoms", alias = "qorx")]
    pub atoms: Vec<RepoAtom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoAtom {
    pub id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub hash: String,
    pub token_estimate: u64,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub signal_mask: u16,
    #[serde(default)]
    pub vector: Vec<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub score: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedContext {
    pub query: String,
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    #[serde(rename = "quarks_used", alias = "atoms_used")]
    pub quarks_used: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackBenchmark {
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub rows: Vec<PackBenchmarkRow>,
    pub average_reduction_x: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackBenchmarkRow {
    pub query: String,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    #[serde(rename = "quarks_used", alias = "atoms_used")]
    pub quarks_used: usize,
}

impl RepoIndex {
    pub fn total_tokens(&self) -> u64 {
        self.atoms.iter().map(|atom| atom.token_estimate).sum()
    }

    pub fn vector_terms(&self) -> usize {
        self.atoms.iter().map(|atom| atom.vector.len()).sum()
    }

    pub fn atom_lookup(&self) -> BTreeMap<&str, &RepoAtom> {
        self.atoms
            .iter()
            .map(|atom| (atom.id.as_str(), atom))
            .collect()
    }
}

pub fn build_index(root: &Path, output: &Path) -> Result<RepoIndex> {
    build_index_with_options(root, output, &IndexOptions::default())
}

pub fn build_index_with_options(
    root: &Path,
    output: &Path,
    options: &IndexOptions,
) -> Result<RepoIndex> {
    let index = build_index_value(root, options)?;
    save_index(&index, output)?;
    Ok(index)
}

pub fn build_index_value(root: &Path, options: &IndexOptions) -> Result<RepoIndex> {
    let root = root
        .canonicalize()
        .with_context(|| format!("could not resolve index root {}", root.display()))?;
    if !root.is_dir() {
        return Err(anyhow!("index root is not a directory: {}", root.display()));
    }

    let mut atoms = Vec::new();
    let mut state = ScanState { indexed_files: 0 };
    scan_dir(&root, &root, &mut atoms, options, &mut state)?;
    atoms.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.id.cmp(&b.id))
    });

    let index = RepoIndex {
        root: root.display().to_string(),
        updated_at: Utc::now(),
        atoms,
    };

    Ok(index)
}

pub fn save_index(index: &RepoIndex, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::proto_store::save(output, index)?;
    Ok(())
}

struct ScanState {
    indexed_files: usize,
}

pub fn load_index(path: &Path) -> Result<RepoIndex> {
    let legacy = path.with_extension("json");
    crate::proto_store::load_required(path, &[legacy.as_path()])
        .with_context(|| format!("could not read Qorx index at {}", path.display()))
}

pub fn search_index(index: &RepoIndex, query: &str, limit: usize) -> Vec<SearchHit> {
    let query_lower = query.to_lowercase();
    let terms = query_terms(&query_lower);
    if terms.is_empty() {
        return Vec::new();
    }
    let query_vector = term_vector(&query_lower);

    let mut hits = index
        .atoms
        .iter()
        .filter_map(|atom| {
            let score = score_atom(atom, &query_lower, &terms, &query_vector);
            if score == 0 {
                return None;
            }
            Some(SearchHit {
                id: atom.id.clone(),
                path: atom.path.clone(),
                start_line: atom.start_line,
                end_line: atom.end_line,
                score,
            })
        })
        .collect::<Vec<_>>();

    hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
    hits.truncate(limit.max(1));
    hits
}

pub fn pack_context(index: &RepoIndex, query: &str, budget_tokens: u64) -> PackedContext {
    let plan = crate::b2c_quant::plan_context(index, query, budget_tokens);
    PackedContext {
        query: plan.query,
        budget_tokens: plan.budget_tokens,
        indexed_tokens: plan.indexed_tokens,
        used_tokens: plan.used_tokens,
        omitted_tokens: plan.omitted_tokens,
        context_reduction_x: plan.context_reduction_x,
        quarks_used: plan.selected_quarks.len(),
        text: plan.text,
    }
}

pub fn benchmark_queries(
    index: &RepoIndex,
    queries: &[String],
    budget_tokens: u64,
) -> PackBenchmark {
    let rows = queries
        .iter()
        .map(|query| {
            let packed = pack_context(index, query, budget_tokens);
            PackBenchmarkRow {
                query: query.clone(),
                used_tokens: packed.used_tokens,
                omitted_tokens: packed.omitted_tokens,
                context_reduction_x: packed.context_reduction_x,
                quarks_used: packed.quarks_used,
            }
        })
        .collect::<Vec<_>>();
    let average_reduction_x = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(|row| row.context_reduction_x).sum::<f64>() / rows.len() as f64
    };

    PackBenchmark {
        budget_tokens: budget_tokens.clamp(128, 20_000),
        indexed_tokens: index.total_tokens(),
        rows,
        average_reduction_x,
    }
}

fn scan_dir(
    root: &Path,
    dir: &Path,
    atoms: &mut Vec<RepoAtom>,
    options: &IndexOptions,
    state: &mut ScanState,
) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if options.lenient => {
            let _ = err;
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };
    for entry in entries {
        if options
            .max_files
            .is_some_and(|max_files| state.indexed_files >= max_files)
        {
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) if options.lenient => {
                let _ = err;
                continue;
            }
            Err(err) => return Err(err.into()),
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if path.is_dir() {
            if should_skip_dir(&name) || should_skip_relative_dir(root, &path) {
                continue;
            }
            scan_dir(root, &path, atoms, options, state)?;
            continue;
        }

        if !should_index_file(&path) {
            continue;
        }
        if !options.include_sensitive && looks_sensitive_path(root, &path) {
            continue;
        }
        match index_file(root, &path, atoms, options.include_sensitive) {
            Ok(true) => state.indexed_files += 1,
            Ok(false) => {}
            Err(err) if options.lenient => {
                let _ = err;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn index_file(
    root: &Path,
    path: &Path,
    atoms: &mut Vec<RepoAtom>,
    include_sensitive: bool,
) -> Result<bool> {
    if is_epub_file(path) {
        return index_epub_file(root, path, atoms);
    }

    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_FILE_BYTES {
        return Ok(false);
    }

    let bytes = fs::read(path)?;
    if bytes.contains(&0) {
        return Ok(false);
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(false);
    };
    if text.trim().is_empty() {
        return Ok(false);
    }
    if !include_sensitive && looks_sensitive_text(&text) {
        return Ok(false);
    }

    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    Ok(index_text_chunks(&relative, &text, atoms))
}

fn index_text_chunks(relative: &str, text: &str, atoms: &mut Vec<RepoAtom>) -> bool {
    let mut indexed = false;
    for (start_line, end_line, chunk) in chunk_text(text) {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let hash = content_hash(relative, start_line, end_line, chunk);
        let id = format!("qva_{}", &hash[..12]);
        let symbols = extract_symbols(chunk);
        let signal_mask = extract_signal_mask(relative, chunk);
        let vector = atom_vector(relative, &symbols, chunk);
        atoms.push(RepoAtom {
            id,
            path: relative.to_string(),
            start_line,
            end_line,
            hash,
            token_estimate: estimate_tokens(chunk),
            symbols,
            signal_mask,
            vector,
            text: chunk.to_string(),
        });
        indexed = true;
    }

    indexed
}

fn is_epub_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_lowercase())
            .as_deref(),
        Some("epub")
    )
}

fn index_epub_file(root: &Path, path: &Path, atoms: &mut Vec<RepoAtom>) -> Result<bool> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_EPUB_BYTES {
        return Ok(false);
    }

    let file = fs::File::open(path)?;
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return Ok(false);
    };
    let base_relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let mut indexed = false;
    let mut text_entries = 0usize;

    for index in 0..archive.len() {
        if text_entries >= MAX_EPUB_TEXT_ENTRIES {
            break;
        }
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() || entry.size() == 0 || entry.size() > MAX_EPUB_ENTRY_BYTES {
            continue;
        }
        let entry_name = entry.name().replace('\\', "/");
        if !should_index_epub_entry(&entry_name) {
            continue;
        }

        let mut bytes = Vec::with_capacity(entry.size().min(MAX_EPUB_ENTRY_BYTES) as usize);
        entry.read_to_end(&mut bytes)?;
        if bytes.contains(&0) {
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let text = normalize_epub_text(&text);
        if text.trim().is_empty() {
            continue;
        }

        let relative = format!("{base_relative}#{entry_name}");
        indexed |= index_text_chunks(&relative, &text, atoms);
        text_entries += 1;
    }

    Ok(indexed)
}

fn should_index_epub_entry(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with("meta-inf/") || lower.starts_with("__macosx/") {
        return false;
    }
    matches!(
        Path::new(&lower)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("xhtml" | "html" | "htm" | "txt")
    )
}

fn normalize_epub_text(text: &str) -> String {
    let without_tags = strip_markup(text);
    let decoded = decode_basic_entities(&without_tags);
    collapse_whitespace(&decoded)
}

fn strip_markup(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => {
                in_tag = true;
                output.push(' ');
            }
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn decode_basic_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn collapse_whitespace(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut previous_space = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !previous_space {
                output.push(' ');
            }
            previous_space = true;
        } else {
            output.push(ch);
            previous_space = false;
        }
    }
    output.trim().to_string()
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".venv"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".next"
            | ".turbo"
            | "__pycache__"
            | ".cache"
            | "cache"
            | ".tmp"
            | "tmp"
            | "log"
            | "logs"
            | ".sandbox-secrets"
            | "$recycle.bin"
            | "paper-qa"
            | "program files"
            | "program files (x86)"
            | "programdata"
            | "recovery"
            | "system volume information"
            | "codex-autoresearch"
            | "papers"
            | "windows"
            | "proc"
            | "sys"
            | "dev"
            | "run"
            | "mnt"
            | "boot"
            | "bin"
            | "sbin"
            | "lib"
            | "lib64"
            | "usr"
            | "var"
    )
}

fn looks_sensitive_path(root: &Path, path: &Path) -> bool {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if matches!(
        name.as_str(),
        ".env"
            | ".env.local"
            | ".npmrc"
            | ".netrc"
            | "id_rsa"
            | "id_dsa"
            | "id_ed25519"
            | "known_hosts"
            | "login data"
            | "cookies"
            | "auth.json"
    ) {
        return true;
    }
    let sensitive_fragments = [
        "secret",
        "secrets",
        "token",
        "tokens",
        "credential",
        "credentials",
        "password",
        "passwd",
        "private",
        "apikey",
        "api_key",
        "api-key",
        "service_account",
        "auth.json",
        "oauth",
        "creds",
    ];
    if sensitive_fragments
        .iter()
        .any(|fragment| relative.contains(fragment))
    {
        return true;
    }
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_lowercase())
            .as_deref(),
        Some("pem" | "key" | "p12" | "pfx" | "cer" | "crt")
    )
}

fn looks_sensitive_text(text: &str) -> bool {
    let lowered = text.to_lowercase();
    [
        "\"access_token\"",
        "\"refresh_token\"",
        "\"id_token\"",
        "\"client_secret\"",
        "\"api_key\"",
        "\"apikey\"",
        "bearer ",
        "-----begin private key-----",
        "-----begin openssh private key-----",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn should_skip_relative_dir(root: &Path, path: &Path) -> bool {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    matches!(relative.as_str(), "docs/assets" | "docs/benchmarks")
}

fn should_index_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if matches!(
        name,
        "Cargo.toml" | "Cargo.lock" | "AGENTS.md" | "README.md" | "package.json"
    ) {
        return true;
    }
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_lowercase())
            .as_deref(),
        Some(
            "rs" | "toml"
                | "md"
                | "json"
                | "jsonl"
                | "qorx"
                | "yaml"
                | "yml"
                | "txt"
                | "epub"
                | "ps1"
                | "py"
                | "js"
                | "mjs"
                | "cjs"
                | "ts"
                | "tsx"
                | "jsx"
                | "html"
                | "css"
                | "scss"
                | "sql"
        )
    )
}

fn chunk_text(text: &str) -> Vec<(usize, usize, String)> {
    let mut chunks = Vec::new();
    let mut start_line = 1;
    let mut current = String::new();
    let mut line_count = 0;

    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        if current.is_empty() {
            start_line = line_no;
        }
        current.push_str(line);
        current.push('\n');
        line_count += 1;

        if line_count >= MAX_CHUNK_LINES || current.len() >= MAX_CHUNK_CHARS {
            chunks.push((start_line, line_no, current.trim_end().to_string()));
            current.clear();
            line_count = 0;
        }
    }

    if !current.trim().is_empty() {
        let end_line = start_line + line_count.saturating_sub(1);
        chunks.push((start_line, end_line, current.trim_end().to_string()));
    }

    chunks
}

fn content_hash(path: &str, start_line: usize, end_line: usize, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(start_line.to_le_bytes());
    hasher.update(end_line.to_le_bytes());
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
        .map(str::trim)
        .filter(|term| term.len() > 1)
        .map(ToOwned::to_owned)
        .collect()
}

fn score_atom(atom: &RepoAtom, query_lower: &str, terms: &[String], query_vector: &[u32]) -> u64 {
    let path_lower = atom.path.to_lowercase();
    let text_lower = atom.text.to_lowercase();
    let mut score = 0;

    if text_lower.contains(query_lower) {
        score += 25;
    }
    if path_lower.contains(query_lower) {
        score += 40;
    }

    for term in terms {
        if path_lower.contains(term) {
            score += 15;
        }
        for symbol in &atom.symbols {
            let symbol_lower = symbol.to_lowercase();
            if symbol_lower == *term {
                score += 35;
            } else if symbol_lower.contains(term) {
                score += 20;
            }
        }
        score += signal_term_score(atom.signal_mask, term);
        let hits = text_lower.matches(term).count() as u64;
        score += hits.min(12).saturating_mul(3);
    }
    score += vector_overlap_score(&atom.vector, query_vector).min(32) * 7;
    score
}

fn atom_vector(path: &str, symbols: &[String], text: &str) -> Vec<u32> {
    let mut material = String::with_capacity(path.len() + text.len() + 64);
    material.push_str(path);
    material.push('\n');
    for symbol in symbols {
        material.push_str(symbol);
        material.push('\n');
    }
    material.push_str(text);
    term_vector(&material.to_lowercase())
}

fn term_vector(text: &str) -> Vec<u32> {
    let mut weights: BTreeMap<u32, u16> = BTreeMap::new();
    for term in query_terms(text) {
        if is_noise_term(&term) {
            continue;
        }
        let hash = hash_term(&term);
        let entry = weights.entry(hash).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    let mut ranked = weights.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(MAX_VECTOR_TERMS);
    let mut vector = ranked.into_iter().map(|(hash, _)| hash).collect::<Vec<_>>();
    vector.sort_unstable();
    vector
}

fn vector_overlap_score(left: &[u32], right: &[u32]) -> u64 {
    let mut score = 0;
    let mut li = 0;
    let mut ri = 0;
    while li < left.len() && ri < right.len() {
        match left[li].cmp(&right[ri]) {
            std::cmp::Ordering::Equal => {
                score += 1;
                li += 1;
                ri += 1;
            }
            std::cmp::Ordering::Less => li += 1,
            std::cmp::Ordering::Greater => ri += 1,
        }
    }
    score
}

fn hash_term(term: &str) -> u32 {
    let mut hash = 2166136261u32;
    for byte in term.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

fn is_noise_term(term: &str) -> bool {
    matches!(
        term,
        "the"
            | "and"
            | "for"
            | "with"
            | "this"
            | "that"
            | "from"
            | "into"
            | "pub"
            | "let"
            | "mut"
            | "use"
            | "fn"
            | "mod"
            | "impl"
            | "true"
            | "false"
            | "none"
            | "null"
            | "self"
            | "return"
    )
}

fn extract_symbols(text: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for line in text.lines() {
        for symbol in symbols_from_line(line) {
            if symbols.len() >= 24 {
                return symbols;
            }
            if !symbols.iter().any(|existing| existing == &symbol) {
                symbols.push(symbol);
            }
        }
    }
    symbols
}

fn symbols_from_line(line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    let probes = [
        "pub fn ",
        "fn ",
        "async fn ",
        "pub struct ",
        "struct ",
        "pub enum ",
        "enum ",
        "pub trait ",
        "trait ",
        "pub mod ",
        "mod ",
        "impl ",
        "def ",
        "class ",
        "function ",
        "export function ",
        "export class ",
        "const ",
        "let ",
        "func ",
        "interface ",
        "type ",
    ];
    let mut out = Vec::new();
    for probe in probes {
        if let Some(rest) = trimmed.strip_prefix(probe) {
            if let Some(symbol) = take_identifier(rest) {
                out.push(symbol);
            }
        }
    }
    out
}

fn extract_signal_mask(path: &str, text: &str) -> u16 {
    let mut mask = signal_mask_for_path(path);
    for line in text.lines() {
        mask |= signal_mask_from_line(line);
    }
    mask
}

fn signal_mask_for_path(path: &str) -> u16 {
    let lower = path.to_lowercase();
    let mut mask = 0;
    if lower.contains("test") || lower.contains("spec") {
        mask |= SIG_TEST;
    }
    if lower.contains("config") || lower.ends_with(".toml") || lower.ends_with(".yaml") {
        mask |= SIG_CONFIG;
    }
    if lower.contains("route") || lower.contains("api") || lower.contains("controller") {
        mask |= SIG_ROUTE;
    }
    mask
}

fn signal_mask_from_line(line: &str) -> u16 {
    let trimmed = line.trim_start();
    let lower = trimmed.to_lowercase();
    let mut mask = 0;

    if lower.starts_with("use ")
        || lower.starts_with("import ")
        || lower.starts_with("from ")
        || lower.starts_with("#include")
        || lower.contains("require(")
    {
        mask |= SIG_IMPORT;
    }
    if lower.contains("route(")
        || lower.contains("router.")
        || lower.contains("app.get")
        || lower.contains("app.post")
        || lower.contains("app.put")
        || lower.contains("app.delete")
        || lower.starts_with("#[get(")
        || lower.starts_with("#[post(")
    {
        mask |= SIG_ROUTE;
    }
    if lower.starts_with("#[test]")
        || lower.starts_with("it(")
        || lower.starts_with("test(")
        || lower.starts_with("describe(")
        || lower.contains("assert_eq!")
        || lower.contains("expect(")
    {
        mask |= SIG_TEST;
    }
    if lower.contains("err(")
        || lower.contains("error")
        || lower.contains("throw ")
        || lower.contains("raise ")
        || lower.contains("panic!")
        || lower.contains("anyhow!")
        || lower.contains("statuscode::")
    {
        mask |= SIG_ERROR;
    }
    if lower.starts_with("if ")
        || lower.starts_with("else ")
        || lower.starts_with("match ")
        || lower.starts_with("for ")
        || lower.starts_with("while ")
        || lower.starts_with("switch ")
        || lower.starts_with("case ")
    {
        mask |= SIG_BRANCH;
    }
    if looks_like_assignment(trimmed) {
        mask |= SIG_ASSIGNMENT;
    }
    if looks_like_call(trimmed) {
        mask |= SIG_CALL;
    }
    if lower.contains("env::var")
        || lower.contains("process.env")
        || lower.contains("getenv")
        || lower.contains("config")
    {
        mask |= SIG_CONFIG;
    }

    mask
}

fn looks_like_assignment(line: &str) -> bool {
    line.contains('=')
        && !line.contains("==")
        && !line.contains("!=")
        && !line.contains("<=")
        && !line.contains(">=")
        && !line.trim_start().starts_with("//")
}

fn looks_like_call(line: &str) -> bool {
    line.contains('(')
        && line.contains(')')
        && !line.trim_start().starts_with("fn ")
        && !line.trim_start().starts_with("def ")
        && !line.trim_start().starts_with("class ")
}

fn signal_term_score(mask: u16, term: &str) -> u64 {
    let mut score = 0;
    if mask & SIG_IMPORT != 0 && signal_term_matches(term, "import") {
        score += 18;
    }
    if mask & SIG_ROUTE != 0 && signal_term_matches(term, "route") {
        score += 18;
    }
    if mask & SIG_TEST != 0 && signal_term_matches(term, "test") {
        score += 18;
    }
    if mask & SIG_ERROR != 0 && signal_term_matches(term, "error") {
        score += 18;
    }
    if mask & SIG_BRANCH != 0 && signal_term_matches(term, "branch") {
        score += 18;
    }
    if mask & SIG_ASSIGNMENT != 0 && signal_term_matches(term, "assignment") {
        score += 18;
    }
    if mask & SIG_CALL != 0 && signal_term_matches(term, "call") {
        score += 18;
    }
    if mask & SIG_CONFIG != 0 && signal_term_matches(term, "config") {
        score += 18;
    }
    score
}

fn signal_term_matches(term: &str, label: &str) -> bool {
    term == label || term.contains(label)
}

fn take_identifier(text: &str) -> Option<String> {
    let text = text.trim_start();
    let ident = text
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '$')
        .collect::<String>();
    if ident.len() > 1 {
        Some(ident)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, path::PathBuf};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        std::env::temp_dir().join(format!("{name}-{suffix}"))
    }

    fn write_epub(path: &Path, files: &[(&str, &str)]) {
        let file = fs::File::create(path).expect("create epub fixture");
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, text) in files {
            zip.start_file(*name, options).expect("start epub entry");
            zip.write_all(text.as_bytes()).expect("write epub entry");
        }
        zip.finish().expect("finish epub fixture");
    }

    #[test]
    fn repo_index_search_finds_relevant_atom() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_test".to_string(),
                path: "src/cache_plan.rs".to_string(),
                start_line: 1,
                end_line: 3,
                hash: "abc".to_string(),
                token_estimate: 12,
                symbols: vec!["split_stable_prefix".to_string()],
                signal_mask: SIG_CALL,
                vector: term_vector("cache plan stable prefix"),
                text: "fn split_stable_prefix() { stable_prefix_tokens(); }".to_string(),
            }],
        };

        let hits = search_index(&index, "cache plan", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "qva_test");
    }

    #[test]
    fn symbol_names_boost_search_and_pack_context() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_symbol".to_string(),
                path: "src/cache.rs".to_string(),
                start_line: 1,
                end_line: 2,
                hash: "abc".to_string(),
                token_estimate: 10,
                symbols: vec!["ExactResponseCache".to_string()],
                signal_mask: SIG_ASSIGNMENT,
                vector: term_vector("ExactResponseCache cache hits"),
                text: "pub struct ExactResponseCache { hits: u64 }".to_string(),
            }],
        };

        let hits = search_index(&index, "ExactResponseCache", 5);
        assert_eq!(hits[0].id, "qva_symbol");
        let packed = pack_context(&index, "ExactResponseCache", 256);
        assert!(packed.text.contains("qva_symbol"));
        assert!(packed.text.contains("symbols=ExactResponseCache"));
        assert_eq!(packed.quarks_used, 1);
        assert!(packed.context_reduction_x > 0.0);
    }

    #[test]
    fn structural_signals_are_extracted_without_heavy_parsers() {
        let mask = extract_signal_mask(
            "src/routes/auth_test.rs",
            "use axum::Router;\n#[test]\nfn login_route_errors() {\n    app.get(\"/login\", handler);\n    let status = StatusCode::BAD_REQUEST;\n    if status == StatusCode::BAD_REQUEST { panic!(\"bad\"); }\n}\n",
        );

        assert_ne!(mask & SIG_IMPORT, 0);
        assert_ne!(mask & SIG_TEST, 0);
        assert_ne!(mask & SIG_ROUTE, 0);
        assert_ne!(mask & SIG_ERROR, 0);
        assert_ne!(mask & SIG_ASSIGNMENT, 0);
        assert_ne!(mask & SIG_BRANCH, 0);
        assert_ne!(mask & SIG_CALL, 0);
    }

    #[test]
    fn structural_signal_terms_match_words_not_prefix_fragments() {
        let mask = SIG_IMPORT | SIG_CONFIG;

        assert!(signal_term_score(mask, "imports") > 0);
        assert!(signal_term_score(mask, "config") > 0);
        assert_eq!(signal_term_score(mask, "imp"), 0);
    }

    #[test]
    fn chunking_preserves_line_ranges() {
        let chunks = chunk_text("one\ntwo\nthree\n");
        assert_eq!(chunks[0].0, 1);
        assert_eq!(chunks[0].1, 3);
        assert!(chunks[0].2.contains("three"));
    }

    #[test]
    fn epub_books_are_indexed_from_xhtml_entries() {
        let root = unique_temp_dir("qorx-epub-index");
        fs::create_dir_all(&root).expect("create test root");
        let epub = root.join("proust.epub");
        write_epub(
            &epub,
            &[
                ("mimetype", "application/epub+zip"),
                (
                    "OEBPS/chapter1.xhtml",
                    r#"<?xml version="1.0"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Combray</h1>
    <p>The madeleine dipped in tea opens involuntary memory.</p>
  </body>
</html>"#,
                ),
            ],
        );

        let index = build_index_value(&root, &IndexOptions::default()).expect("index epub");
        let packed = pack_context(&index, "madeleine tea Combray", 400);

        assert!(index.total_tokens() > 0);
        assert!(index
            .atoms
            .iter()
            .any(|atom| atom.path == "proust.epub#OEBPS/chapter1.xhtml"));
        assert!(packed.text.contains("madeleine"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sparse_vectors_match_related_terms_without_dense_embeddings() {
        let left = term_vector("turboquant kv cache quantization prompt cache");
        let right = term_vector("kv cache adapter turboquant");
        assert!(vector_overlap_score(&left, &right) >= 2);
    }

    #[test]
    fn generated_doc_assets_and_benchmarks_are_not_indexed() {
        let root = Path::new(r"C:\repo");

        assert!(should_skip_relative_dir(
            root,
            Path::new(r"C:\repo\docs\assets")
        ));
        assert!(should_skip_relative_dir(
            root,
            Path::new(r"C:\repo\docs\benchmarks")
        ));
        assert!(!should_skip_relative_dir(
            root,
            Path::new(r"C:\repo\docs\papers")
        ));
    }

    #[test]
    fn sensitive_tokens_are_not_indexed_even_when_path_looks_safe() {
        let root = unique_temp_dir("qorx-sensitive-content");
        fs::create_dir_all(root.join("accounts/user")).expect("create account dir");
        fs::write(
            root.join("accounts/user/profile.json"),
            r#"{"name":"marvin","access_token":"secret","refresh_token":"secret"}"#,
        )
        .expect("write sensitive json");
        fs::write(
            root.join("notes.md"),
            "safe public note about context routing",
        )
        .expect("write safe note");

        let index = build_index_value(&root, &IndexOptions::default()).expect("index root");

        assert!(index.atoms.iter().any(|atom| atom.path == "notes.md"));
        assert!(!index
            .atoms
            .iter()
            .any(|atom| atom.path == "accounts/user/profile.json"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn benchmark_reports_reduction_for_each_query() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_bench".to_string(),
                path: "src/index.rs".to_string(),
                start_line: 1,
                end_line: 2,
                hash: "abc".to_string(),
                token_estimate: 10,
                symbols: vec!["pack_context".to_string()],
                signal_mask: SIG_CALL,
                vector: term_vector("pack context benchmark"),
                text: "pub fn pack_context() {}".to_string(),
            }],
        };
        let report = benchmark_queries(&index, &["pack context".to_string()], 256);
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].quarks_used, 1);
    }
}
