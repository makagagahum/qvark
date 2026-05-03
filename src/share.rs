use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capsule::{self, Capsule},
    config::AppPaths,
    lattice::{self, LatticeRules, LatticeState},
};

const FEDERATION_FILE: &str = "qorx-federation.pb";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareBundle {
    pub schema: String,
    pub created_at: String,
    pub qorx_version: String,
    pub transport: String,
    pub lattice: LatticeState,
    pub rules: Option<LatticeRules>,
    pub capsule: Option<Capsule>,
    pub bundle_sha256: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareExportReport {
    pub schema: String,
    pub out: String,
    pub bytes: u64,
    pub transport: String,
    pub lattice: String,
    pub nodes: usize,
    pub capsules: usize,
    pub bundle_sha256: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareImportReport {
    pub schema: String,
    pub source: String,
    pub transport: String,
    pub imported_nodes: usize,
    pub federated_capsules: usize,
    pub merged_lattice: LatticeState,
    pub federation: FederationState,
    pub local_only: bool,
    pub provider_calls: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationState {
    pub schema: String,
    pub handle: String,
    pub created_at: String,
    pub transport: String,
    pub bundles: Vec<FederatedBundle>,
    pub capsules: Vec<Capsule>,
    pub merged_lattice_handle: String,
    pub prompt_block: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedBundle {
    pub source_lattice: String,
    pub nodes: usize,
    pub capsules: usize,
    pub bundle_sha256: String,
}

pub fn federation_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(FEDERATION_FILE)
}

pub fn export(paths: &AppPaths, out: &Path) -> Result<ShareExportReport> {
    let lattice = lattice::load(paths)?;
    let rules = lattice::load_rules(paths).ok();
    let capsule = capsule::load(paths).ok();
    let mut bundle = ShareBundle {
        schema: "qorx.share.bundle.v1".to_string(),
        created_at: Utc::now().to_rfc3339(),
        qorx_version: env!("CARGO_PKG_VERSION").to_string(),
        transport: "local_file".to_string(),
        lattice,
        rules,
        capsule,
        bundle_sha256: String::new(),
        boundary: "Qorx share bundles are local protobuf files for moving lattice/capsule state between trusted machines or agent workspaces. Import performs deterministic set-union by node id and edge hash; it does not provide network sync, conflict resolution, ordering guarantees, remote access, or cloud federation.".to_string(),
    };
    bundle.bundle_sha256 = bundle_hash(&bundle)?;
    crate::proto_store::save(out, &bundle)?;
    let bytes = fs::metadata(out).map(|meta| meta.len()).unwrap_or(0);
    Ok(ShareExportReport {
        schema: "qorx.share.export.v1".to_string(),
        out: out.display().to_string(),
        bytes,
        transport: "local_file".to_string(),
        lattice: bundle.lattice.handle.clone(),
        nodes: bundle.lattice.nodes.len(),
        capsules: usize::from(bundle.capsule.is_some()),
        bundle_sha256: bundle.bundle_sha256,
        local_only: true,
        provider_calls: 0,
        boundary: "Export writes a local protobuf bundle only. Inspect the bundle before sharing outside the machine; this is file-based share, not network federation.".to_string(),
    })
}

pub fn export_capsule(
    paths: &AppPaths,
    capsule_handle: Option<&str>,
    out: &Path,
) -> Result<ShareExportReport> {
    if let Some(expected) = capsule_handle {
        let capsule = capsule::load(paths)?;
        if capsule.handle != expected {
            return Err(anyhow!(
                "requested capsule `{expected}` does not match active capsule `{}`",
                capsule.handle
            ));
        }
    }
    export(paths, out)
}

pub fn import(paths: &AppPaths, bundle_path: &Path) -> Result<ShareImportReport> {
    let bundle: ShareBundle = crate::proto_store::load_required(bundle_path, &[])?;
    let expected = bundle_hash_without_field(&bundle)?;
    if !bundle.bundle_sha256.is_empty() && bundle.bundle_sha256 != expected {
        return Err(anyhow!("share bundle hash mismatch"));
    }

    let local = lattice::load(paths).ok();
    let merged = merge_lattices(local.as_ref(), &bundle.lattice)?;
    crate::proto_store::save(&lattice::lattice_path(paths), &merged)?;
    if let Some(rules) = &bundle.rules {
        crate::proto_store::save(&lattice::rules_path(paths), rules)?;
    }

    let capsules = bundle.capsule.clone().into_iter().collect::<Vec<_>>();
    let federation = build_federation(&merged, &bundle, capsules);
    crate::proto_store::save(&federation_path(paths), &federation)?;
    Ok(ShareImportReport {
        schema: "qorx.share.import.v1".to_string(),
        source: bundle_path.display().to_string(),
        transport: "local_file".to_string(),
        imported_nodes: bundle.lattice.nodes.len(),
        federated_capsules: federation.capsules.len(),
        merged_lattice: merged,
        federation,
        local_only: true,
        provider_calls: 0,
        boundary: "Import merges a local protobuf share bundle into this Qorx home by node id and edge hash. It is file-based share/merge, not cloud sync, remote execution, or conflict-resolved federation.".to_string(),
    })
}

pub fn session(paths: &AppPaths) -> Result<FederationState> {
    crate::proto_store::load_required(&federation_path(paths), &[])
}

fn merge_lattices(local: Option<&LatticeState>, incoming: &LatticeState) -> Result<LatticeState> {
    let mut merged = local.cloned().unwrap_or_else(|| incoming.clone());
    if local.is_some() {
        let mut known = merged
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for node in &incoming.nodes {
            if known.insert(node.id.clone()) {
                merged.nodes.push(node.clone());
            }
        }
        let mut known_edges = merged
            .edges
            .iter()
            .map(|edge| edge.edge_sha256.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for edge in &incoming.edges {
            if known_edges.insert(edge.edge_sha256.clone()) {
                merged.edges.push(edge.clone());
            }
        }
    }
    merged.created_at = Utc::now().to_rfc3339();
    merged.task = format!("federated: {}", merged.task);
    let seed = serde_json::to_vec(&serde_json::json!({
        "nodes": &merged.nodes,
        "edges": &merged.edges,
        "task": &merged.task,
    }))?;
    merged.handle = format!("qorx://l/{}", &hex_sha256(&seed)[..16]);
    merged.prompt_block = format!(
        "QORX_LATTICE {}\nlayers=4 nodes={} q={} local_idx={}\nmode=local-file-merged-lattice local_pb.\n{}",
        merged.handle,
        merged.nodes.len(),
        merged.nodes.iter().filter(|node| node.layer == 0).count(),
        merged.b2c.local_idx_tokens,
        merged.b2c.proof_tail
    );
    Ok(merged)
}

fn build_federation(
    merged: &LatticeState,
    bundle: &ShareBundle,
    capsules: Vec<Capsule>,
) -> FederationState {
    let created_at = Utc::now().to_rfc3339();
    let seed = format!(
        "{}\n{}\n{}\n{}",
        merged.handle,
        bundle.lattice.handle,
        bundle.bundle_sha256,
        capsules.len()
    );
    let handle = format!("qorx://f/{}", &hex_sha256(seed.as_bytes())[..16]);
    let prompt_block = format!(
        "QORX_FEDERATION {handle}\ntransport=local_file bundles=1 capsules={} nodes={}\nmode=local-file-share local_pb; deterministic set-union merge; resolve with Qorx.\nproof at={} lattice={} bundle={}",
        capsules.len(),
        merged.nodes.len(),
        created_at,
        merged.handle,
        &bundle.bundle_sha256[..16.min(bundle.bundle_sha256.len())],
    );
    FederationState {
        schema: "qorx.federation.v1".to_string(),
        handle,
        created_at,
        transport: "local_file".to_string(),
        bundles: vec![FederatedBundle {
            source_lattice: bundle.lattice.handle.clone(),
            nodes: bundle.lattice.nodes.len(),
            capsules: capsules.len(),
            bundle_sha256: bundle.bundle_sha256.clone(),
        }],
        capsules,
        merged_lattice_handle: merged.handle.clone(),
        prompt_block,
        local_only: true,
        provider_calls: 0,
        boundary: "Federation state is local file exchange only. It records deterministic set-union merges by node id and edge hash; remote transport, peer discovery, conflict resolution, and network trust management are outside the core.".to_string(),
    }
}

fn bundle_hash(bundle: &ShareBundle) -> Result<String> {
    bundle_hash_without_field(bundle)
}

fn bundle_hash_without_field(bundle: &ShareBundle) -> Result<String> {
    let mut clone = bundle.clone();
    clone.bundle_sha256.clear();
    Ok(hex_sha256(&serde_json::to_vec(&clone)?))
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
