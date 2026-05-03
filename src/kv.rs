use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{config::AppPaths, lattice};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvEmitReport {
    pub schema: String,
    pub model: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub realized_kv_compression: bool,
    pub hints: Vec<lattice::KvHint>,
    pub safetensors: SafeTensorWrite,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeTensorWrite {
    pub written: bool,
    pub path: Option<String>,
    pub bytes: u64,
    pub format: String,
}

pub fn emit(
    paths: &AppPaths,
    model: &str,
    task: Option<&str>,
    out: Option<PathBuf>,
) -> Result<KvEmitReport> {
    let export = lattice::kv_hint_export(paths, task)?;
    let safetensors = if let Some(out) = out {
        let bytes = safetensors_bytes(model, &export.hints)?;
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out, &bytes)?;
        SafeTensorWrite {
            written: true,
            path: Some(out.display().to_string()),
            bytes: bytes.len() as u64,
            format: "safetensors-u8-manifest".to_string(),
        }
    } else {
        SafeTensorWrite {
            written: false,
            path: None,
            bytes: 0,
            format: "safetensors-u8-manifest".to_string(),
        }
    };

    Ok(KvEmitReport {
        schema: "qorx.kv.emit.v1".to_string(),
        model: model.to_string(),
        local_only: true,
        provider_calls: 0,
        realized_kv_compression: export.realized_kv_compression,
        hints: export.hints,
        safetensors,
        boundary: "KV emit writes a safetensors-compatible U8 hint manifest for external adapters. It is not a runtime KV tensor dump and does not prove KV-cache compression until a compatible local runtime consumes the hints and reports memory or latency data.".to_string(),
    })
}

fn safetensors_bytes(model: &str, hints: &[lattice::KvHint]) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut header = serde_json::Map::new();
    header.insert(
        "__metadata__".to_string(),
        serde_json::json!({
            "format": "qorx-kv-hints",
            "model": model,
            "realized_kv_compression": "false",
        }),
    );
    for hint in hints {
        let payload = serde_json::to_vec(hint)?;
        let start = data.len();
        data.extend_from_slice(&payload);
        let end = data.len();
        header.insert(
            safe_tensor_name(&hint.id),
            serde_json::json!({
                "dtype": "U8",
                "shape": [payload.len()],
                "data_offsets": [start, end],
            }),
        );
    }
    let header_bytes = serde_json::to_vec(&serde_json::Value::Object(header))?;
    let mut out = Vec::with_capacity(8 + header_bytes.len() + data.len());
    out.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&data);
    Ok(out)
}

fn safe_tensor_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
