use std::{env, fs, path::PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimReport {
    pub found: bool,
    pub path: Option<String>,
    pub bytes: u64,
    pub fingerprint: Option<String>,
    pub metadata: Option<AimMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimMetadata {
    pub r#type: Option<String>,
    pub version: Option<String>,
    pub mode: Option<String>,
    pub source: Option<String>,
    pub workspace_root: Option<String>,
    pub tensor_dim: Option<u64>,
    pub chunks: Option<u64>,
    pub files: Option<u64>,
    pub generated_at: Option<u64>,
    pub security: Option<String>,
}

pub fn inspect_default() -> Result<AimReport> {
    inspect_path(resolve_aim_path())
}

pub fn resolve_aim_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("QORX_AIM_PATH").map(PathBuf::from) {
        return Some(path);
    }
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|home| {
            home.join("Documents")
                .join("brain")
                .join(".aim")
                .join("memory.aim")
        })
        .filter(|path| path.exists())
}

pub fn inspect_path(path: Option<PathBuf>) -> Result<AimReport> {
    let Some(path) = path else {
        return Ok(AimReport {
            found: false,
            path: None,
            bytes: 0,
            fingerprint: None,
            metadata: None,
        });
    };
    if !path.exists() {
        return Ok(AimReport {
            found: false,
            path: Some(path.display().to_string()),
            bytes: 0,
            fingerprint: None,
            metadata: None,
        });
    }

    let bytes = fs::read(&path)?;
    let metadata = parse_metadata(&bytes).ok();
    Ok(AimReport {
        found: true,
        path: Some(path.display().to_string()),
        bytes: bytes.len() as u64,
        fingerprint: Some(hex_sha256(&bytes)),
        metadata,
    })
}

pub fn parse_metadata(bytes: &[u8]) -> Result<AimMetadata> {
    let prefix = b"AIMTTT";
    if !bytes.starts_with(prefix) {
        return Err(anyhow!("AIM file missing AIMTTT header"));
    }
    let rest = &bytes[prefix.len()..];
    let end =
        json_object_end(rest).ok_or_else(|| anyhow!("sidecar metadata JSON is incomplete"))?;
    Ok(serde_json::from_slice(&rest[..end])?)
}

fn json_object_end(bytes: &[u8]) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, byte) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_aim_header_metadata() {
        let bytes = br#"AIMTTT{"chunks":244,"files":200,"mode":"local-sidecar","security":"integrity-watermark","source":"qorx-local-build","tensor_dim":1536,"type":"qorx_aim_sidecar","version":"2026.1","workspace_root":"C:\\repo"}binary"#;

        let metadata = super::parse_metadata(bytes).unwrap();

        assert_eq!(metadata.tensor_dim, Some(1536));
        assert_eq!(metadata.chunks, Some(244));
        assert_eq!(metadata.security.as_deref(), Some("integrity-watermark"));
    }
}
