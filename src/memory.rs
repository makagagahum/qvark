use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{compression::estimate_tokens, config::AppPaths};

const MEMORY_FILE: &str = "marvin-memory.pb";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub summary: String,
    pub token_estimate: u64,
    pub hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReport {
    pub schema: String,
    pub action: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub item: Option<MemoryItem>,
    pub items: Vec<MemoryItem>,
    pub summary: String,
    pub deleted: bool,
    pub items_kept: usize,
    pub pruned_count: usize,
    pub omitted_tokens: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryStore {
    schema: String,
    updated_at: String,
    items: Vec<MemoryItem>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            schema: "qorx.marvin-memory-store.v1".to_string(),
            updated_at: Utc::now().to_rfc3339(),
            items: Vec::new(),
        }
    }
}

pub fn memory_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(MEMORY_FILE)
}

pub fn create(paths: &AppPaths, kind: &str, text: &str) -> Result<MemoryReport> {
    let path = memory_path(paths);
    let mut store = load_store(&path)?;
    let item = new_item(kind, text);
    store.items.push(item.clone());
    save_store(&path, &mut store)?;
    Ok(report("create", Some(item), Vec::new()))
}

pub fn read(paths: &AppPaths, query: &str, limit: usize) -> Result<MemoryReport> {
    let store = load_store(&memory_path(paths))?;
    let terms = terms(query);
    let mut ranked = store
        .items
        .into_iter()
        .filter_map(|item| {
            let score = score_item(&item, &terms);
            (score > 0).then_some((score, item))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.updated_at.cmp(&a.1.updated_at)));
    let items = ranked
        .into_iter()
        .take(limit.clamp(1, 32))
        .map(|(_, item)| item)
        .collect::<Vec<_>>();
    Ok(report("read", None, items))
}

pub fn update(paths: &AppPaths, id: &str, text: &str) -> Result<MemoryReport> {
    let path = memory_path(paths);
    let mut store = load_store(&path)?;
    let Some(item) = store.items.iter_mut().find(|item| item.id == id) else {
        return Err(anyhow!("memory item not found: {id}"));
    };
    let hash = hash_text(text);
    item.text = text.trim().to_string();
    item.summary = summarize_text(text);
    item.token_estimate = estimate_tokens(text);
    item.hash = hash;
    item.updated_at = Utc::now().to_rfc3339();
    let item = item.clone();
    save_store(&path, &mut store)?;
    Ok(report("update", Some(item), Vec::new()))
}

pub fn delete(paths: &AppPaths, id: &str) -> Result<MemoryReport> {
    let path = memory_path(paths);
    let mut store = load_store(&path)?;
    let before = store.items.len();
    store.items.retain(|item| item.id != id);
    let deleted = store.items.len() != before;
    save_store(&path, &mut store)?;
    let mut report = report("delete", None, Vec::new());
    report.deleted = deleted;
    Ok(report)
}

pub fn summarize(paths: &AppPaths, limit: usize) -> Result<MemoryReport> {
    let mut items = load_store(&memory_path(paths))?.items;
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    items.truncate(limit.clamp(1, 32));
    let summary = items
        .iter()
        .map(|item| format!("{}:{} {}", item.kind, item.id, item.summary))
        .collect::<Vec<_>>()
        .join("\n");
    let mut report = report("summarize", None, items);
    report.summary = summary;
    Ok(report)
}

pub fn read_all(paths: &AppPaths) -> Result<Vec<MemoryItem>> {
    Ok(load_store(&memory_path(paths))?.items)
}

pub fn prune(paths: &AppPaths, max_items: usize) -> Result<MemoryReport> {
    let path = memory_path(paths);
    let mut store = load_store(&path)?;
    let before = store.items.len();
    store.items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let max_items = max_items.max(1);
    let omitted_tokens = store
        .items
        .iter()
        .skip(max_items)
        .map(|item| item.token_estimate)
        .sum();
    store.items.truncate(max_items);
    let kept = store.items.len();
    let pruned_count = before.saturating_sub(kept);
    save_store(&path, &mut store)?;
    let mut report = report("prune", None, store.items);
    report.items_kept = kept;
    report.pruned_count = pruned_count;
    report.omitted_tokens = omitted_tokens;
    Ok(report)
}

pub fn gc(paths: &AppPaths, strategy: &str, max_items: usize) -> Result<MemoryReport> {
    if !strategy.eq_ignore_ascii_case("lattice") {
        return Err(anyhow!(
            "unsupported memory gc strategy `{strategy}`; supported: lattice"
        ));
    }
    let path = memory_path(paths);
    let mut store = load_store(&path)?;
    let before = store.items.len();
    store.items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let mut seen = BTreeSet::new();
    let mut kept = Vec::new();
    let mut omitted_tokens = 0;
    let old_items = std::mem::take(&mut store.items);
    for item in old_items {
        let key = format!("{}\n{}", item.kind, hash_text(&item.text));
        if seen.insert(key) && kept.len() < max_items.max(1) {
            kept.push(item);
        } else {
            omitted_tokens += item.token_estimate;
        }
    }

    let kept_count = kept.len();
    let pruned_count = before.saturating_sub(kept_count);
    store.items = kept;
    save_store(&path, &mut store)?;
    let mut report = report("gc", None, store.items);
    report.items_kept = kept_count;
    report.pruned_count = pruned_count;
    report.omitted_tokens = omitted_tokens;
    report.boundary = "Lattice GC deduplicates local memory cards and prunes the memory working set while preserving raw quark evidence in the repository/capsule index. It performs no provider calls.".to_string();
    Ok(report)
}

fn load_store(path: &Path) -> Result<MemoryStore> {
    let legacy = path.with_extension("json");
    crate::proto_store::load_or_default(path, &[legacy.as_path()])
}

fn save_store(path: &Path, store: &mut MemoryStore) -> Result<()> {
    store.updated_at = Utc::now().to_rfc3339();
    crate::proto_store::save(path, store)
}

fn new_item(kind: &str, text: &str) -> MemoryItem {
    let now = Utc::now().to_rfc3339();
    let text = text.trim().to_string();
    let hash = hash_text(&text);
    let id_seed = hash_text(&format!("{}\n{text}\n{now}", normalize_kind(kind)));
    MemoryItem {
        id: format!("qvm_{}", &id_seed[..12]),
        kind: normalize_kind(kind),
        summary: summarize_text(&text),
        token_estimate: estimate_tokens(&text),
        hash,
        text,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn report(action: &str, item: Option<MemoryItem>, items: Vec<MemoryItem>) -> MemoryReport {
    MemoryReport {
        schema: "qorx.memory.v1".to_string(),
        action: action.to_string(),
        local_only: true,
        provider_calls: 0,
        item,
        items,
        summary: String::new(),
        deleted: false,
        items_kept: 0,
        pruned_count: 0,
        omitted_tokens: 0,
        boundary: "Marvin memory is local protobuf-envelope CRUD state. It stores compact text cards and summaries; model training, hidden context, and unsupported fact certification are outside this memory store.".to_string(),
    }
}

fn score_item(item: &MemoryItem, query_terms: &[String]) -> usize {
    if query_terms.is_empty() {
        return 0;
    }
    let haystack = format!("{} {} {}", item.kind, item.summary, item.text).to_lowercase();
    query_terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count()
}

fn terms(text: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else {
            push_term(&mut terms, &mut current);
        }
    }
    push_term(&mut terms, &mut current);
    terms.into_iter().collect()
}

fn push_term(terms: &mut BTreeSet<String>, current: &mut String) {
    if current.len() >= 3 {
        terms.insert(std::mem::take(current));
    }
    current.clear();
}

fn normalize_kind(kind: &str) -> String {
    let normalized = kind
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect::<String>()
        .to_ascii_lowercase();
    if normalized.is_empty() {
        "note".to_string()
    } else {
        normalized
    }
}

fn summarize_text(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= 180 {
        return text.to_string();
    }
    let mut out = text.chars().take(177).collect::<String>();
    out.push_str("...");
    out
}

fn hash_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn kind_normalization_keeps_empty_kind_safe() {
        assert_eq!(super::normalize_kind("Decision!"), "decision");
        assert_eq!(super::normalize_kind("!!!"), "note");
    }

    #[test]
    fn created_memory_hash_is_content_fingerprint_not_id_seed() {
        let item = super::new_item("decision", "route only measured savings");

        assert_eq!(item.hash, super::hash_text("route only measured savings"));
        assert_ne!(item.id, format!("qvm_{}", &item.hash[..12]));
    }
}
