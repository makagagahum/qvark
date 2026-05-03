use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AtomStore {
    #[serde(default, rename = "quarks", alias = "atoms")]
    pub atoms: BTreeMap<String, String>,
}

impl AtomStore {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let legacy = legacy_quark_paths(path);
        crate::proto_store::load_or_default(
            path,
            &legacy.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        )
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        crate::proto_store::save(path.as_ref(), self)
    }

    fn intern(&mut self, text: &str) -> (String, bool) {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        let key = format!("qvk_{}", &hash[..12]);
        let is_new = !self.atoms.contains_key(&key);
        if is_new {
            self.atoms.insert(key.clone(), text.to_string());
        }
        (key, is_new)
    }
}

fn legacy_quark_paths(path: &Path) -> Vec<PathBuf> {
    vec![
        path.with_extension("json"),
        path.with_file_name("atoms.json"),
    ]
}

#[derive(Debug, Clone)]
pub struct CompressionReport {
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub quarks_created: u64,
}

#[derive(Debug, Clone)]
struct TextCompression {
    text: String,
    quarks_created: u64,
}

pub const TOKEN_ESTIMATOR_LABEL: &str = "char4";

pub fn estimate_tokens(text: &str) -> u64 {
    ((text.chars().count() as f64) / 4.0).ceil().max(1.0) as u64
}

pub fn compress_json_body(body: &[u8], store: &mut AtomStore) -> (Vec<u8>, CompressionReport) {
    let raw_text = String::from_utf8_lossy(body);
    let raw_tokens = estimate_tokens(&raw_text);
    let Ok(mut value) = serde_json::from_slice::<Value>(body) else {
        return (
            body.to_vec(),
            CompressionReport {
                raw_tokens,
                compressed_tokens: raw_tokens,
                quarks_created: 0,
            },
        );
    };

    let mut quarks_created = 0;
    compress_value(&mut value, store, &mut quarks_created);

    let compressed = serde_json::to_vec(&value).unwrap_or_else(|_| body.to_vec());
    let compressed_text = String::from_utf8_lossy(&compressed);
    let compressed_tokens = estimate_tokens(&compressed_text);

    (
        compressed,
        CompressionReport {
            raw_tokens,
            compressed_tokens,
            quarks_created,
        },
    )
}

fn compress_value(value: &mut Value, store: &mut AtomStore, quarks_created: &mut u64) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if should_compress_key(key) || child.is_object() || child.is_array() {
                    compress_value(child, store, quarks_created);
                }
            }
        }
        Value::Array(items) => {
            for child in items {
                compress_value(child, store, quarks_created);
            }
        }
        Value::String(text) => {
            if text.len() < 900 {
                return;
            }
            let compressed = compress_text(text, store);
            if compressed.text.len() < text.len() {
                *text = compressed.text;
                *quarks_created += compressed.quarks_created;
            }
        }
        _ => {}
    }
}

fn should_compress_key(key: &str) -> bool {
    matches!(
        key,
        "content" | "text" | "input" | "prompt" | "system" | "developer" | "instructions"
    )
}

fn compress_text(text: &str, store: &mut AtomStore) -> TextCompression {
    let normalized = normalize_whitespace(text);
    let blocks = split_blocks(&normalized);
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for block in &blocks {
        if block.len() >= 240 {
            *counts.entry(block.as_str()).or_insert(0) += 1;
        }
    }

    let mut atom_table = Vec::new();
    let mut atom_keys: HashMap<String, String> = HashMap::new();
    let mut quarks_created = 0;

    for (block, count) in counts {
        if count < 2 {
            continue;
        }
        let (key, is_new) = store.intern(block);
        if is_new {
            quarks_created += 1;
        }
        atom_keys.insert(block.to_string(), key.clone());
        atom_table.push(format!("{key}: {block}"));
    }

    if atom_keys.is_empty() {
        return TextCompression {
            text: normalized,
            quarks_created,
        };
    }

    let mut rewritten = Vec::with_capacity(blocks.len() + 1);
    rewritten.push(format!(
        "[Qorx quark table]\n{}\n[/Qorx quark table]",
        atom_table.join("\n")
    ));

    for block in blocks {
        if let Some(key) = atom_keys.get(block.as_str()) {
            rewritten.push(format!("[Qorx quark:{key}]"));
        } else {
            rewritten.push(block);
        }
    }

    TextCompression {
        text: rewritten.join("\n\n"),
        quarks_created,
    }
}

fn normalize_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_lines = 0;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_lines += 1;
            if blank_lines <= 1 {
                out.push('\n');
            }
        } else {
            blank_lines = 0;
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

fn split_blocks(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_large_blocks_are_replaced_with_quarks() {
        let repeated = "alpha ".repeat(80);
        let text = format!("{repeated}\n\nmiddle\n\n{repeated}");
        let mut store = AtomStore::default();
        let compressed = compress_text(&text, &mut store);

        assert!(compressed.text.contains("[Qorx quark table]"));
        assert!(compressed.text.contains("[Qorx quark:qvk_"));
        assert_eq!(compressed.quarks_created, 1);
    }
}
