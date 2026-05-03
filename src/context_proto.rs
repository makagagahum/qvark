use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use chrono::Utc;
use prost::Message;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compression::AtomStore,
    config::AppPaths,
    index::{load_index, RepoIndex},
};

const SCHEMA: &str = "qorx.state-snapshot.protobuf.v1";

#[derive(Clone, PartialEq, Message)]
struct ContextSnapshotPb {
    #[prost(string, tag = "1")]
    schema: String,
    #[prost(string, tag = "2")]
    created_at: String,
    #[prost(string, tag = "3")]
    qorx_version: String,
    #[prost(string, tag = "4")]
    data_dir: String,
    #[prost(uint64, tag = "5")]
    indexed_tokens: u64,
    #[prost(uint64, tag = "6")]
    index_quarks: u64,
    #[prost(uint64, tag = "7")]
    quark_store_entries: u64,
    #[prost(message, optional, tag = "8")]
    repo_index: Option<RepoIndexPb>,
    #[prost(message, repeated, tag = "9")]
    quark_store: Vec<QuarkStoreEntryPb>,
    #[prost(message, repeated, tag = "10")]
    files: Vec<ContextFilePb>,
    #[prost(string, tag = "11")]
    boundary: String,
}

#[derive(Clone, PartialEq, Message)]
struct RepoIndexPb {
    #[prost(string, tag = "1")]
    root: String,
    #[prost(string, tag = "2")]
    updated_at: String,
    #[prost(message, repeated, tag = "3")]
    quarks: Vec<RepoQuarkPb>,
}

#[derive(Clone, PartialEq, Message)]
struct RepoQuarkPb {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    path: String,
    #[prost(uint64, tag = "3")]
    start_line: u64,
    #[prost(uint64, tag = "4")]
    end_line: u64,
    #[prost(string, tag = "5")]
    hash: String,
    #[prost(uint64, tag = "6")]
    token_estimate: u64,
    #[prost(string, repeated, tag = "7")]
    symbols: Vec<String>,
    #[prost(uint32, tag = "8")]
    signal_mask: u32,
    #[prost(uint32, repeated, tag = "9")]
    vector: Vec<u32>,
    #[prost(string, tag = "10")]
    text: String,
}

#[derive(Clone, PartialEq, Message)]
struct QuarkStoreEntryPb {
    #[prost(string, tag = "1")]
    key: String,
    #[prost(string, tag = "2")]
    text: String,
}

#[derive(Clone, PartialEq, Message)]
struct ContextFilePb {
    #[prost(string, tag = "1")]
    logical_name: String,
    #[prost(string, tag = "2")]
    path: String,
    #[prost(bool, tag = "3")]
    present: bool,
    #[prost(string, tag = "4")]
    sha256: String,
    #[prost(bytes, tag = "5")]
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextProtoReport {
    pub schema: String,
    pub snapshot_path: String,
    pub saved: bool,
    pub verified: bool,
    pub coverage_percent: f64,
    pub files_checked: usize,
    pub protobuf_bytes: u64,
    pub stored_file_bytes: u64,
    pub indexed_tokens: u64,
    pub index_quarks: usize,
    pub quark_store_entries: usize,
    pub files: Vec<ContextProtoFileReport>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextProtoFileReport {
    pub logical_name: String,
    pub path: String,
    pub present: bool,
    pub sha256: String,
    pub matches_disk: bool,
    pub bytes: u64,
}

pub fn snapshot(paths: &AppPaths) -> Result<ContextProtoReport> {
    let index = load_index(&paths.index_file)?;
    let quarks = AtomStore::load(&paths.atom_file)?;
    let files = context_files(paths)?;
    let snapshot = ContextSnapshotPb {
        schema: SCHEMA.to_string(),
        created_at: Utc::now().to_rfc3339(),
        qorx_version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: paths.data_dir.display().to_string(),
        indexed_tokens: index.total_tokens(),
        index_quarks: index.atoms.len() as u64,
        quark_store_entries: quarks.atoms.len() as u64,
        repo_index: Some(repo_index_pb(&index)),
        quark_store: quarks
            .atoms
            .iter()
            .map(|(key, text)| QuarkStoreEntryPb {
                key: key.clone(),
                text: text.clone(),
            })
            .collect(),
        files,
        boundary: boundary(),
    };

    let mut encoded = Vec::new();
    snapshot.encode(&mut encoded)?;
    if let Some(parent) = paths.context_protobuf_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&paths.context_protobuf_file, encoded)?;
    verify(paths).map(|mut report| {
        report.saved = true;
        report
    })
}

pub fn verify(paths: &AppPaths) -> Result<ContextProtoReport> {
    let bytes = fs::read(&paths.context_protobuf_file).map_err(|err| {
        anyhow!(
            "could not read Qorx protobuf context snapshot at {}: {err}",
            paths.context_protobuf_file.display()
        )
    })?;
    let snapshot = ContextSnapshotPb::decode(bytes.as_slice())?;
    let mut files = Vec::new();
    let mut matches = 0usize;
    let mut stored_file_bytes = 0u64;

    for file in &snapshot.files {
        stored_file_bytes += file.bytes.len() as u64;
        let matches_disk = file_matches_disk(file);
        if matches_disk {
            matches += 1;
        }
        files.push(ContextProtoFileReport {
            logical_name: file.logical_name.clone(),
            path: file.path.clone(),
            present: file.present,
            sha256: file.sha256.clone(),
            matches_disk,
            bytes: file.bytes.len() as u64,
        });
    }

    let files_checked = files.len();
    let coverage_percent = if files_checked == 0 {
        0.0
    } else {
        (matches as f64 / files_checked as f64) * 100.0
    };
    let typed_index_matches = snapshot
        .repo_index
        .as_ref()
        .is_some_and(|index| index.quarks.len() as u64 == snapshot.index_quarks);
    let typed_store_matches = snapshot.quark_store.len() as u64 == snapshot.quark_store_entries;
    let verified = coverage_percent == 100.0 && typed_index_matches && typed_store_matches;

    Ok(ContextProtoReport {
        schema: snapshot.schema,
        snapshot_path: paths.context_protobuf_file.display().to_string(),
        saved: true,
        verified,
        coverage_percent,
        files_checked,
        protobuf_bytes: bytes.len() as u64,
        stored_file_bytes,
        indexed_tokens: snapshot.indexed_tokens,
        index_quarks: snapshot.index_quarks as usize,
        quark_store_entries: snapshot.quark_store_entries as usize,
        files,
        boundary: snapshot.boundary,
    })
}

fn context_files(paths: &AppPaths) -> Result<Vec<ContextFilePb>> {
    let marvin_memory = crate::memory::memory_path(paths);
    let lattice = crate::lattice::lattice_path(paths);
    let lattice_rules = crate::lattice::rules_path(paths);
    let federation = crate::share::federation_path(paths);
    let files = vec![
        ("repo_index", paths.index_file.as_path()),
        ("quarks", paths.atom_file.as_path()),
        ("response_cache", paths.response_cache_file.as_path()),
        ("stats", paths.stats_file.as_path()),
        ("integrations", paths.integration_report_file.as_path()),
        ("provenance", paths.provenance_file.as_path()),
        ("marvin_memory", marvin_memory.as_path()),
        ("lattice", lattice.as_path()),
        ("lattice_rules", lattice_rules.as_path()),
        ("federation", federation.as_path()),
    ];
    files
        .into_iter()
        .map(|(logical_name, path)| file_pb(logical_name, path))
        .collect()
}

fn file_pb(logical_name: &str, path: &Path) -> Result<ContextFilePb> {
    if !path.exists() {
        migrate_legacy_json(path)?;
    }
    if !path.exists() {
        return Ok(ContextFilePb {
            logical_name: logical_name.to_string(),
            path: path.display().to_string(),
            present: false,
            sha256: String::new(),
            bytes: Vec::new(),
        });
    }
    let bytes = fs::read(path)?;
    Ok(ContextFilePb {
        logical_name: logical_name.to_string(),
        path: path.display().to_string(),
        present: true,
        sha256: hex_sha256(&bytes),
        bytes,
    })
}

fn migrate_legacy_json(path: &Path) -> Result<()> {
    let legacy = path.with_extension("json");
    if !legacy.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(&legacy)?;
    let value = serde_json::from_str::<serde_json::Value>(&text)?;
    crate::proto_store::save(path, &value)
}

fn file_matches_disk(file: &ContextFilePb) -> bool {
    let path = Path::new(&file.path);
    if !file.present {
        return !path.exists();
    }
    let Ok(current) = fs::read(path) else {
        return false;
    };
    current == file.bytes && hex_sha256(&current) == file.sha256
}

fn repo_index_pb(index: &RepoIndex) -> RepoIndexPb {
    RepoIndexPb {
        root: index.root.clone(),
        updated_at: index.updated_at.to_rfc3339(),
        quarks: index
            .atoms
            .iter()
            .map(|quark| RepoQuarkPb {
                id: quark.id.clone(),
                path: quark.path.clone(),
                start_line: quark.start_line as u64,
                end_line: quark.end_line as u64,
                hash: quark.hash.clone(),
                token_estimate: quark.token_estimate,
                symbols: quark.symbols.clone(),
                signal_mask: quark.signal_mask as u32,
                vector: quark.vector.clone(),
                text: quark.text.clone(),
            })
            .collect(),
    }
}

fn boundary() -> String {
    "The protobuf snapshot is a local backup and verification artifact for Qorx-owned non-secret state: repo index, quark store, response cache, stats, integrations, provenance, Marvin memory, lattice, lattice rules, and local-file share state. It is not a model-visible context pack. Hidden model context, provider payloads, and private signing seeds are outside this snapshot boundary.".to_string()
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
