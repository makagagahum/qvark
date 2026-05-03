use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    compression::estimate_tokens,
    config::AppPaths,
    index::{self, RepoAtom, RepoIndex},
    memory,
};

const LATTICE_FILE: &str = "qorx-lattice.pb";
const RULES_FILE: &str = "qorx-lattice-rules.pb";
const DEFAULT_BUDGET_TOKENS: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeState {
    pub schema: String,
    pub qorx_version: String,
    pub created_at: String,
    pub task: String,
    pub handle: String,
    pub prompt_block: String,
    pub layers: Vec<LatticeLayer>,
    pub nodes: Vec<LatticeNode>,
    pub edges: Vec<LatticeEdge>,
    pub b2c: LatticeB2cProof,
    pub kv_hints: KvHintExport,
    pub local_only: bool,
    pub provider_calls: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeLayer {
    pub layer: u8,
    pub name: String,
    pub nodes: usize,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeNode {
    pub id: String,
    pub layer: u8,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub token_estimate: u64,
    pub provenance: Vec<String>,
    pub content_sha256: String,
    pub b2c_tail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub edge_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeB2cProof {
    pub local_idx_tokens: u64,
    pub visible_tokens: u64,
    pub saved_tokens: u64,
    pub reduction_x: f64,
    pub proof_tail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvHintExport {
    pub schema: String,
    pub runtime_targets: Vec<String>,
    pub hints: Vec<KvHint>,
    pub realized_kv_compression: bool,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvHint {
    pub id: String,
    pub target_node: String,
    pub source_quarks: Vec<String>,
    pub estimated_tokens: u64,
    pub cache_key: String,
    pub quantization_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvolveReport {
    pub schema: String,
    pub task: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub lattice: LatticeState,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeAttestation {
    pub schema: String,
    pub formal: bool,
    pub verified: bool,
    pub provider_calls: u64,
    pub lattice: String,
    pub certificate_sha256: String,
    pub checks: Vec<AttestationCheck>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalAttestation {
    pub schema: String,
    pub formal: bool,
    pub level: u8,
    pub verified: bool,
    pub provider_calls: u64,
    pub lattice: LatticeAttestation,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeRules {
    pub schema: String,
    pub generation: u64,
    pub updated_at: String,
    pub task: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub metrics: RuleMetrics,
    pub rules: Vec<LatticeRule>,
    pub rules_protobuf: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    pub provenance_edges: usize,
    pub cross_layer_edges: usize,
    pub coherence_score: f64,
    pub entropy_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeRule {
    pub name: String,
    pub enabled: bool,
    pub threshold: f64,
    pub reason: String,
    pub rule_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub fn lattice_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(LATTICE_FILE)
}

pub fn rules_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(RULES_FILE)
}

pub fn evolve(paths: &AppPaths, task: &str, budget_tokens: u64) -> Result<MemoryEvolveReport> {
    let lattice = build(paths, task, budget_tokens)?;
    crate::proto_store::save(&lattice_path(paths), &lattice)?;
    Ok(MemoryEvolveReport {
        schema: "qorx.memory.evolve.v1".to_string(),
        task: task.to_string(),
        local_only: true,
        provider_calls: 0,
        lattice,
        boundary: "Memory evolve builds deterministic lattice mementos from exact local quarks and memory cards. Provider calls and facts absent from provenance are outside this local evolution step.".to_string(),
    })
}

pub fn build(paths: &AppPaths, task: &str, budget_tokens: u64) -> Result<LatticeState> {
    let index = index::load_index(&paths.index_file)?;
    let memory_items = memory::read_all(paths).unwrap_or_default();
    let task = normalize_task(task);
    let selected = selected_quarks(&index, &task);
    let selected_ids = selected
        .iter()
        .map(|atom| atom.id.clone())
        .collect::<Vec<_>>();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for atom in &selected {
        nodes.push(quark_node(atom));
    }

    let vector = vector_node(&task, &selected);
    for quark_id in &selected_ids {
        edges.push(edge(&vector.id, quark_id, "indexes"));
    }
    nodes.push(vector.clone());

    let memento = memento_node(&task, &selected, &memory_items);
    edges.push(edge(&memento.id, &vector.id, "abstracts"));
    for quark_id in &selected_ids {
        edges.push(edge(&memento.id, quark_id, "provenance"));
    }
    nodes.push(memento.clone());

    let strategy = strategy_node(&task, &memento, &selected);
    edges.push(edge(&strategy.id, &memento.id, "plans_with"));
    nodes.push(strategy);

    let layers = layer_report(&nodes);
    let local_idx_tokens = index.total_tokens()
        + memory_items
            .iter()
            .map(|item| item.token_estimate)
            .sum::<u64>();
    let handle_seed = serde_json::to_vec(&serde_json::json!({
        "task": task,
        "nodes": &nodes,
        "edges": &edges,
        "local_idx_tokens": local_idx_tokens,
        "budget_tokens": budget_tokens.max(128),
    }))?;
    let handle = format!("qorx://l/{}", &hex_sha256(&handle_seed)[..16]);
    let created_at = Utc::now().to_rfc3339();
    let proof_seed = LatticeB2cProof {
        local_idx_tokens,
        visible_tokens: 1,
        saved_tokens: local_idx_tokens.saturating_sub(1),
        reduction_x: local_idx_tokens.max(1) as f64,
        proof_tail: String::new(),
    };
    let mut block = prompt_block(
        &handle,
        &created_at,
        nodes.len(),
        selected_ids.len(),
        local_idx_tokens,
        estimate_tokens(&handle),
        proof_seed.reduction_x,
    );
    let visible_tokens = estimate_tokens(&block);
    let saved_tokens = local_idx_tokens.saturating_sub(visible_tokens);
    let reduction_x = local_idx_tokens.max(1) as f64 / visible_tokens.max(1) as f64;
    block = prompt_block(
        &handle,
        &created_at,
        nodes.len(),
        selected_ids.len(),
        local_idx_tokens,
        visible_tokens,
        reduction_x,
    );
    let proof_tail = format!(
        "proof at={} ctx={}t vis={}t saved={}t qshf={:.2}x est={} b2c=accounting",
        created_at,
        local_idx_tokens,
        visible_tokens,
        saved_tokens,
        reduction_x,
        crate::compression::TOKEN_ESTIMATOR_LABEL
    );
    let b2c = LatticeB2cProof {
        local_idx_tokens,
        visible_tokens,
        saved_tokens,
        reduction_x,
        proof_tail,
    };
    let kv_hints = kv_hints(&handle, &nodes, &selected_ids, budget_tokens);

    Ok(LatticeState {
        schema: "qorx.lattice.v1".to_string(),
        qorx_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at,
        task,
        handle,
        prompt_block: block,
        layers,
        nodes,
        edges,
        b2c,
        kv_hints,
        local_only: true,
        provider_calls: 0,
        boundary: "Qorx Lattice is deterministic local protobuf state. Higher layers are extractive mementos with immutable provenance to raw quarks; KV output is a runtime hint export until a runtime reports live KV tensor measurements.".to_string(),
    })
}

pub fn load(paths: &AppPaths) -> Result<LatticeState> {
    crate::proto_store::load_required(&lattice_path(paths), &[])
}

pub fn status(paths: &AppPaths) -> Result<LatticeState> {
    load(paths)
}

pub fn attest(paths: &AppPaths, formal: bool) -> Result<LatticeAttestation> {
    let lattice = load(paths)?;
    let checks = verify_lattice(&lattice);
    let verified = checks.iter().all(|check| check.passed);
    let certificate = serde_json::to_vec(&serde_json::json!({
        "formal": formal,
        "lattice": lattice.handle,
        "checks": checks,
        "b2c": lattice.b2c,
        "nodes": lattice.nodes.len(),
        "edges": lattice.edges.len(),
    }))?;
    Ok(LatticeAttestation {
        schema: "qorx.lattice.attestation.v1".to_string(),
        formal,
        verified,
        provider_calls: 0,
        lattice: lattice.handle,
        certificate_sha256: hex_sha256(&certificate),
        checks,
        boundary: "Formal lattice attestation is a local machine-checkable certificate over node hashes, provenance links, edge hashes, layer direction, and B2C arithmetic. It is not an external proof of world truth.".to_string(),
    })
}

pub fn formal_attest(paths: &AppPaths, formal: bool, level: u8) -> Result<FormalAttestation> {
    let lattice = attest(paths, formal)?;
    Ok(FormalAttestation {
        schema: "qorx.formal-attestation.v1".to_string(),
        formal,
        level: level.clamp(1, 3),
        verified: lattice.verified,
        provider_calls: 0,
        lattice,
        boundary: "Qorx formal attestation checks the local lattice, provenance direction, hashes, and B2C arithmetic. It is machine-checkable local evidence, not an external theorem prover or world-truth oracle.".to_string(),
    })
}

pub fn kv_hint_export(paths: &AppPaths, task: Option<&str>) -> Result<KvHintExport> {
    let lattice = match load(paths) {
        Ok(lattice) => lattice,
        Err(_) => build(
            paths,
            task.unwrap_or("local runtime context hint manifest"),
            DEFAULT_BUDGET_TOKENS,
        )?,
    };
    Ok(lattice.kv_hints)
}

pub fn evolve_rules(paths: &AppPaths, task: &str) -> Result<LatticeRules> {
    let lattice = match load(paths) {
        Ok(lattice) => lattice,
        Err(_) => build(paths, task, DEFAULT_BUDGET_TOKENS)?,
    };
    let previous = load_rules(paths).ok();
    let generation = previous.as_ref().map_or(1, |rules| rules.generation + 1);
    let metrics = rule_metrics(&lattice);
    let mut rules = vec![
        make_rule(
            "promote_multi_source_mementos",
            metrics.provenance_edges >= 2,
            2.0,
            "Promote mementos only when they retain at least two provenance edges.",
        ),
        make_rule(
            "demote_low_coherence_strategy",
            metrics.coherence_score < 0.42,
            0.42,
            "Demote strategy nodes when edge density and provenance coherence are weak.",
        ),
        make_rule(
            "gc_duplicate_memory_hashes",
            true,
            1.0,
            "Collapse duplicate local memory cards by kind and text hash before lattice reuse.",
        ),
        make_rule(
            "prefer_exact_quark_sources",
            true,
            1.0,
            "Prefer raw quark provenance over unsupported generated summaries.",
        ),
    ];
    rules.sort_by(|a, b| a.name.cmp(&b.name));
    let mut report = LatticeRules {
        schema: "qorx.lattice.rules.v1".to_string(),
        generation,
        updated_at: Utc::now().to_rfc3339(),
        task: normalize_task(task),
        local_only: true,
        provider_calls: 0,
        metrics,
        rules,
        rules_protobuf: rules_path(paths).display().to_string(),
        boundary: "Lattice rules evolve deterministically from local lattice metrics. They do not call an LLM, train a model, or rewrite raw quark evidence.".to_string(),
    };
    stamp_rule_hashes(&mut report);
    crate::proto_store::save(&rules_path(paths), &report)?;
    Ok(report)
}

pub fn load_rules(paths: &AppPaths) -> Result<LatticeRules> {
    crate::proto_store::load_required(&rules_path(paths), &[])
}

fn selected_quarks(index: &RepoIndex, task: &str) -> Vec<RepoAtom> {
    let hits = index::search_index(index, task, 8);
    let atoms_by_id = index.atom_lookup();
    let mut selected = hits
        .into_iter()
        .filter_map(|hit| atoms_by_id.get(hit.id.as_str()).map(|atom| (*atom).clone()))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        selected = index.atoms.iter().take(8).cloned().collect();
    }
    selected
}

fn quark_node(atom: &RepoAtom) -> LatticeNode {
    let title = format!("{}:{}-{}", atom.path, atom.start_line, atom.end_line);
    let summary = first_line(&atom.text);
    node(
        atom.id.clone(),
        0,
        "quark",
        title,
        summary,
        atom.token_estimate,
        Vec::new(),
    )
}

fn vector_node(task: &str, selected: &[RepoAtom]) -> LatticeNode {
    let mut terms = BTreeSet::new();
    for term in terms_from(task) {
        terms.insert(term);
    }
    for atom in selected {
        for symbol in &atom.symbols {
            terms.insert(symbol.to_ascii_lowercase());
        }
    }
    let summary = terms.into_iter().take(24).collect::<Vec<_>>().join(" ");
    let provenance = selected
        .iter()
        .map(|atom| atom.id.clone())
        .collect::<Vec<_>>();
    let id = format!("qvl_vec_{}", &hex_sha256(summary.as_bytes())[..12]);
    let token_estimate = estimate_tokens(&summary).max(1);
    node(
        id,
        1,
        "semantic-index",
        "deterministic sparse semantic index".to_string(),
        summary,
        token_estimate,
        provenance,
    )
}

fn memento_node(
    task: &str,
    selected: &[RepoAtom],
    memory_items: &[memory::MemoryItem],
) -> LatticeNode {
    let mut lines = Vec::new();
    lines.push(format!("task={task}"));
    for atom in selected.iter().take(4) {
        lines.push(format!("{} -> {}", atom.path, first_line(&atom.text)));
    }
    for item in memory_items.iter().take(4) {
        lines.push(format!("memory:{} -> {}", item.kind, item.summary));
    }
    let summary = lines.join(" | ");
    let provenance = selected
        .iter()
        .map(|atom| atom.id.clone())
        .collect::<Vec<_>>();
    let id = format!("qvl_mem_{}", &hex_sha256(summary.as_bytes())[..12]);
    node(
        id,
        2,
        "memento",
        format!("memento for {task}"),
        summary,
        selected
            .iter()
            .map(|atom| atom.token_estimate)
            .sum::<u64>()
            .min(240),
        provenance,
    )
}

fn strategy_node(task: &str, memento: &LatticeNode, selected: &[RepoAtom]) -> LatticeNode {
    let paths = selected
        .iter()
        .map(|atom| atom.path.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(8)
        .collect::<Vec<_>>()
        .join(", ");
    let summary = format!("Use {} with exact evidence paths: {}", memento.id, paths);
    let id = format!(
        "qvl_strat_{}",
        &hex_sha256(format!("{task}\n{summary}").as_bytes())[..12]
    );
    node(
        id,
        3,
        "strategy",
        format!("strategy for {task}"),
        summary,
        64,
        vec![memento.id.clone()],
    )
}

fn node(
    id: String,
    layer: u8,
    kind: &str,
    title: String,
    summary: String,
    token_estimate: u64,
    provenance: Vec<String>,
) -> LatticeNode {
    let content_sha256 = node_hash(layer, kind, &title, &summary, &provenance);
    let b2c_tail = format!(
        "node={} layer={} local_tokens={} proof={}",
        id,
        layer,
        token_estimate,
        &content_sha256[..16]
    );
    LatticeNode {
        id,
        layer,
        kind: kind.to_string(),
        title,
        summary,
        token_estimate,
        provenance,
        content_sha256,
        b2c_tail,
    }
}

fn edge(from: &str, to: &str, relation: &str) -> LatticeEdge {
    LatticeEdge {
        from: from.to_string(),
        to: to.to_string(),
        relation: relation.to_string(),
        edge_sha256: edge_hash(from, to, relation),
    }
}

fn layer_report(nodes: &[LatticeNode]) -> Vec<LatticeLayer> {
    let mut counts = BTreeMap::<u8, usize>::new();
    for node in nodes {
        *counts.entry(node.layer).or_default() += 1;
    }
    [
        (
            0,
            "raw quarks",
            "Exact extractive evidence nodes from the local repo index.",
        ),
        (
            1,
            "sparse semantic index",
            "Deterministic lexical/symbol signal layer; no hidden embedding model is required.",
        ),
        (
            2,
            "mementos",
            "Consolidated deterministic claims that retain provenance edges to raw quarks.",
        ),
        (
            3,
            "strategy",
            "Cross-evidence planning nodes used as a tiny active-context surface.",
        ),
    ]
    .into_iter()
    .map(|(layer, name, boundary)| LatticeLayer {
        layer,
        name: name.to_string(),
        nodes: counts.get(&layer).copied().unwrap_or(0),
        boundary: boundary.to_string(),
    })
    .collect()
}

fn kv_hints(
    handle: &str,
    nodes: &[LatticeNode],
    selected_quarks: &[String],
    budget_tokens: u64,
) -> KvHintExport {
    let hints =
        nodes
            .iter()
            .filter(|node| node.layer >= 2)
            .map(|node| {
                let seed = format!("{handle}\n{}\n{}", node.id, node.content_sha256);
                KvHint {
                    id: format!("qvkh_{}", &hex_sha256(seed.as_bytes())[..12]),
                    target_node: node.id.clone(),
                    source_quarks: if node.provenance.is_empty() {
                        selected_quarks.to_vec()
                    } else {
                        node.provenance.clone()
                    },
                    estimated_tokens: node.token_estimate.min(budget_tokens.max(128)),
                    cache_key: hex_sha256(seed.as_bytes()),
                    quantization_hint:
                        "3.5-bit-channel target for compatible local KV runtimes; hint only"
                            .to_string(),
                }
            })
            .collect::<Vec<_>>();
    KvHintExport {
        schema: "qorx.lattice.kv-hints.v1".to_string(),
        runtime_targets: vec![
            "llama.cpp-compatible-external-adapter".to_string(),
            "vllm-compatible-external-adapter".to_string(),
            "sglang-compatible-external-adapter".to_string(),
        ],
        hints,
        realized_kv_compression: false,
        boundary: "Qorx exports deterministic KV hint manifests. The safetensors file stores U8 JSON hint payloads, not runtime KV tensors; actual KV tensor compression is reported only after a local inference runtime consumes the hints and measures cache compression.".to_string(),
    }
}

fn verify_lattice(lattice: &LatticeState) -> Vec<AttestationCheck> {
    let node_ids = lattice
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.layer))
        .collect::<BTreeMap<_, _>>();
    let mut checks = Vec::new();
    checks.push(check(
        "node_hashes",
        lattice.nodes.iter().all(|node| {
            node.content_sha256
                == node_hash(
                    node.layer,
                    &node.kind,
                    &node.title,
                    &node.summary,
                    &node.provenance,
                )
        }),
        "all node content hashes recompute from canonical node fields",
    ));
    checks.push(check(
        "edge_hashes",
        lattice
            .edges
            .iter()
            .all(|edge| edge.edge_sha256 == edge_hash(&edge.from, &edge.to, &edge.relation)),
        "all edge hashes recompute from source, target, and relation",
    ));
    checks.push(check(
        "provenance_targets",
        lattice.nodes.iter().all(|node| {
            node.provenance
                .iter()
                .all(|id| node_ids.contains_key(id.as_str()))
        }),
        "all provenance ids point to local lattice nodes",
    ));
    checks.push(check(
        "layer_direction",
        lattice.edges.iter().all(|edge| {
            let from = node_ids.get(edge.from.as_str()).copied().unwrap_or(0);
            let to = node_ids.get(edge.to.as_str()).copied().unwrap_or(0);
            from >= to
        }),
        "edges flow from consolidated layers back to same-or-lower evidence layers",
    ));
    checks.push(check(
        "b2c_math",
        lattice.b2c.saved_tokens
            == lattice
                .b2c
                .local_idx_tokens
                .saturating_sub(lattice.b2c.visible_tokens)
            && lattice.b2c.reduction_x >= 1.0,
        "visible, saved, and reduction fields are internally consistent",
    ));
    checks.push(check(
        "local_only",
        lattice.local_only && lattice.provider_calls == 0,
        "lattice construction records no provider calls",
    ));
    checks
}

fn rule_metrics(lattice: &LatticeState) -> RuleMetrics {
    let provenance_edges = lattice
        .edges
        .iter()
        .filter(|edge| edge.relation == "provenance")
        .count();
    let layers = lattice
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.layer))
        .collect::<BTreeMap<_, _>>();
    let cross_layer_edges = lattice
        .edges
        .iter()
        .filter(|edge| {
            let from = layers.get(edge.from.as_str()).copied().unwrap_or(0);
            let to = layers.get(edge.to.as_str()).copied().unwrap_or(0);
            from != to
        })
        .count();
    let node_count = lattice.nodes.len();
    let edge_count = lattice.edges.len();
    let max_edges = node_count.max(1) * node_count.max(1);
    let edge_density = edge_count as f64 / max_edges as f64;
    let provenance_ratio = provenance_edges as f64 / edge_count.max(1) as f64;
    let layer_ratio = cross_layer_edges as f64 / edge_count.max(1) as f64;
    let coherence_score =
        ((edge_density * 0.25) + (provenance_ratio * 0.45) + (layer_ratio * 0.30)).clamp(0.0, 1.0);
    let entropy_score = (1.0 - coherence_score).clamp(0.0, 1.0);
    RuleMetrics {
        node_count,
        edge_count,
        provenance_edges,
        cross_layer_edges,
        coherence_score,
        entropy_score,
    }
}

fn make_rule(name: &str, enabled: bool, threshold: f64, reason: &str) -> LatticeRule {
    LatticeRule {
        name: name.to_string(),
        enabled,
        threshold,
        reason: reason.to_string(),
        rule_sha256: String::new(),
    }
}

fn stamp_rule_hashes(report: &mut LatticeRules) {
    for rule in &mut report.rules {
        rule.rule_sha256 = hex_sha256(
            serde_json::to_vec(&serde_json::json!({
                "generation": report.generation,
                "task": report.task,
                "name": rule.name,
                "enabled": rule.enabled,
                "threshold": rule.threshold,
                "reason": rule.reason,
            }))
            .unwrap_or_default()
            .as_slice(),
        );
    }
}

fn check(name: &str, passed: bool, detail: &str) -> AttestationCheck {
    AttestationCheck {
        name: name.to_string(),
        passed,
        detail: detail.to_string(),
    }
}

fn prompt_block(
    handle: &str,
    created_at: &str,
    nodes: usize,
    quarks: usize,
    local_idx_tokens: u64,
    visible_tokens: u64,
    reduction_x: f64,
) -> String {
    let saved_tokens = local_idx_tokens.saturating_sub(visible_tokens);
    format!(
        "QORX_LATTICE {handle}\nlayers=4 nodes={nodes} q={quarks} local_idx={local_idx_tokens}\nmode=deterministic-lattice local_pb; resolve with Qorx lattice.\nproof at={created_at} ctx={local_idx_tokens}t vis={visible_tokens}t saved={saved_tokens}t qshf={reduction_x:.2}x est={} b2c=accounting",
        crate::compression::TOKEN_ESTIMATOR_LABEL
    )
}

fn node_hash(layer: u8, kind: &str, title: &str, summary: &str, provenance: &[String]) -> String {
    hex_sha256(
        serde_json::to_vec(&serde_json::json!({
            "layer": layer,
            "kind": kind,
            "title": title,
            "summary": summary,
            "provenance": provenance,
        }))
        .unwrap_or_default()
        .as_slice(),
    )
}

fn edge_hash(from: &str, to: &str, relation: &str) -> String {
    hex_sha256(format!("{from}\n{to}\n{relation}").as_bytes())
}

fn normalize_task(task: &str) -> String {
    let trimmed = task.trim();
    if trimmed.is_empty() {
        "local context task".to_string()
    } else {
        trimmed.to_string()
    }
}

fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let mut out = line.chars().take(180).collect::<String>();
    if line.chars().count() > 180 {
        out.push_str("...");
    }
    out
}

fn terms_from(text: &str) -> Vec<String> {
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

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn missing_lattice_error() -> anyhow::Error {
    anyhow!(
        "Qorx lattice state not found. Run `qorx memory evolve --task \"...\"` or `qorx lattice build --task \"...\"` first."
    )
}
