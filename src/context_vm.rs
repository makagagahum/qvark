use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    compression::{estimate_tokens, TOKEN_ESTIMATOR_LABEL},
    config::LOCAL_BASE,
    index::{pack_context, RepoIndex},
    session::{build_session_pointer, SessionPointer},
    squeeze::{squeeze_context, SqueezedEvidence},
    stats::Pricing,
    truth::{strict_answer, StrictEvidence},
};

pub const CONTEXT_VM_VERSION: &str = "1.0.0";
pub const QUETTA_ALIAS: &str = "Q";
const SYNTHETIC_CONTEXT_BYTES: &str = "1000000000000000000000000000000";
const COUNTERFACTUAL_VALUE_USD: &str = "1000000000000000000000000000000000";
const VISIBLE_ALIAS_COST_USD: &str = "0.001";
const COUNTERFACTUAL_LEVERAGE_X: &str = "1000000000000000000000000000000000000";

#[derive(Debug, Clone, Copy)]
pub struct ContextVmOptions {
    pub budget_tokens: u64,
    pub limit: usize,
}

impl ContextVmOptions {
    pub fn normalized(self) -> Self {
        Self {
            budget_tokens: self.budget_tokens.clamp(128, 20_000),
            limit: self.limit.clamp(1, 16),
        }
    }
}

impl Default for ContextVmOptions {
    fn default() -> Self {
        Self {
            budget_tokens: 900,
            limit: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVmReport {
    pub schema: String,
    pub version: String,
    pub objective: String,
    pub mode: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub session: SessionPointer,
    pub capabilities: Vec<VmCapability>,
    pub contract: VmToolContract,
    pub prompt_block: String,
    pub plan: Vec<VmStep>,
    pub page_faults: Vec<ContextFaultReport>,
    pub proof_pages: Vec<ProofPage>,
    pub ledger: VmLedger,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFaultReport {
    pub schema: String,
    pub version: String,
    pub fault_id: String,
    pub handle: String,
    pub carrier: Option<String>,
    pub query: String,
    pub status: String,
    pub authorized: bool,
    pub authorization: String,
    pub resolver: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub used_tokens: u64,
    pub avoided_context_tokens: u64,
    pub context_reduction_x: f64,
    pub proof_pages: Vec<ProofPage>,
    pub missing_terms: Vec<String>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInjectReport {
    pub schema: String,
    pub version: String,
    pub objective: String,
    pub handle: String,
    pub gateway: String,
    pub vm_endpoint: String,
    pub fault_endpoint: String,
    pub additional_context: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub budget_tokens: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextNanoReport {
    pub schema: String,
    pub version: String,
    pub objective: String,
    pub carrier: String,
    pub handle: String,
    pub visible_tokens: u64,
    pub indexed_tokens: u64,
    pub avoided_context_tokens: u64,
    pub context_reduction_x: f64,
    pub local_only: bool,
    pub provider_calls: u64,
    pub fault_endpoint: String,
    pub expand_endpoint: String,
    pub budget_tokens: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQuettaReport {
    pub schema: String,
    pub version: String,
    pub objective: String,
    pub carrier: String,
    pub handle: String,
    pub visible_tokens: u64,
    pub local_indexed_tokens: u64,
    pub manifest: QuettaManifest,
    pub value_ledger: QuettaValueLedger,
    pub local_only: bool,
    pub provider_calls: u64,
    pub fault_endpoint: String,
    pub expand_endpoint: String,
    pub budget_tokens: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuettaManifest {
    pub schema: String,
    pub alias: String,
    pub handle: String,
    pub root_fingerprint: String,
    pub manifest_hash: String,
    pub quark_count: usize,
    pub indexed_tokens: u64,
    pub logical_context_bytes: String,
    pub logical_context_unit: String,
    pub physical_manifest_present: bool,
    pub token_estimator: String,
    pub proof_mode: String,
    pub lossless_resolver: bool,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuettaValueLedger {
    pub schema: String,
    pub counterfactual_value_usd: String,
    pub visible_alias_cost_usd: String,
    pub effective_leverage_x: String,
    pub billing_claim: bool,
    pub accounting_mode: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextExpandReport {
    pub schema: String,
    pub version: String,
    pub carrier: String,
    pub handle: String,
    pub status: String,
    pub authorized: bool,
    pub authorization: String,
    pub contract: VmToolContract,
    pub manifest: Option<QuettaManifest>,
    pub additional_context: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPage {
    pub uri: String,
    pub source_kind: String,
    pub quark_id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub excerpt_hash: String,
    pub content_hash: String,
    pub support: String,
    pub matched_terms: Vec<String>,
    pub excerpt_tokens: u64,
    pub excerpt: String,
    pub resolver: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCapability {
    pub handle: String,
    pub kind: String,
    pub permissions: Vec<String>,
    pub resolver: String,
    pub budget_tokens: u64,
    pub ttl: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmToolContract {
    pub gateway: String,
    pub vm_endpoint: String,
    pub fault_endpoint: String,
    pub request_shape: serde_json::Value,
    pub prompt_policy: String,
    pub subagent_policy: String,
    pub unsupported_policy: String,
    pub billing_policy: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStep {
    pub action: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmLedger {
    pub indexed_tokens: u64,
    pub visible_frame_tokens: u64,
    pub proof_tokens: u64,
    pub sent_tokens: u64,
    pub avoided_context_tokens: u64,
    pub context_reduction_x: f64,
    pub estimated_usd_saved: f64,
    pub pricing: Pricing,
    pub estimator: String,
    pub boundary: String,
}

pub fn build_context_vm(
    index: &RepoIndex,
    objective: &str,
    options: ContextVmOptions,
) -> ContextVmReport {
    let options = options.normalized();
    let session = build_session_pointer(index);
    let fault = resolve_context_fault(index, &session.handle, objective, options);
    let packed = pack_context(index, objective, options.budget_tokens);
    let proof_pages = fault.proof_pages.clone();
    let frame = format!(
        "QORX_CONTEXT_VM version={CONTEXT_VM_VERSION} handle={} objective={} budget={} proofs={}",
        session.handle,
        objective,
        options.budget_tokens,
        proof_pages.len()
    );
    let visible_frame_tokens = estimate_tokens(&frame).max(1);
    let proof_tokens = proof_pages
        .iter()
        .map(|page| page.excerpt_tokens + 20)
        .sum::<u64>();
    let sent_tokens = visible_frame_tokens + proof_tokens;
    let ledger = ledger(index.total_tokens(), visible_frame_tokens, proof_tokens);
    let capabilities = capabilities(&session.handle, options.budget_tokens);
    let contract = tool_contract(&session.handle, options.budget_tokens, options.limit);
    let prompt_block = prompt_block(&session, objective, &contract, options.budget_tokens);
    let plan = vec![
        VmStep {
            action: "session".to_string(),
            status: "ready".to_string(),
            reason: "emit a tiny qorx:// session handle before any bulk context".to_string(),
        },
        VmStep {
            action: "strict-answer".to_string(),
            status: if fault.missing_terms.is_empty() {
                "supported".to_string()
            } else {
                "checked".to_string()
            },
            reason: "try extractive support first, then refuse or continue to a page fault"
                .to_string(),
        },
        VmStep {
            action: "page-fault".to_string(),
            status: fault.status.clone(),
            reason: "resolve the objective into cited proof pages under budget".to_string(),
        },
        VmStep {
            action: "working-set".to_string(),
            status: "planned".to_string(),
            reason: format!(
                "pack remains available for larger local working sets; current pack would use {} tokens",
                packed.used_tokens
            ),
        },
        VmStep {
            action: "ledger".to_string(),
            status: "recordable".to_string(),
            reason: format!(
                "account for {} local tokens avoided from the model-visible frame and proof pages",
                ledger.avoided_context_tokens
            ),
        },
    ];

    ContextVmReport {
        schema: "qorx.context-vm.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        objective: objective.to_string(),
        mode: "handle_first_context_virtual_machine_alpha".to_string(),
        local_only: true,
        provider_calls: 0,
        session,
        capabilities,
        contract,
        prompt_block,
        plan,
        page_faults: vec![fault],
        proof_pages,
        ledger: VmLedger {
            sent_tokens,
            ..ledger
        },
        boundary: "Qorx Context VM is a local context-memory manager, not a Linux virtual machine. It keeps Qorx-known context in Cosmos, exposes scoped handles, resolves proof pages on demand, and reports fresh model-visible context avoided instead of claiming that tokens disappear.".to_string(),
    }
}

pub fn resolve_context_fault(
    index: &RepoIndex,
    handle: &str,
    query: &str,
    options: ContextVmOptions,
) -> ContextFaultReport {
    let options = options.normalized();
    let authorization = authorize_handle(index, handle);
    if !authorization.authorized {
        return unauthorized_fault(index, handle, query, options, authorization);
    }
    let resolved_handle = authorization.resolved_handle.clone();
    let carrier = authorization.carrier.clone();

    let strict = strict_answer(index, query, options.limit.min(4));
    let squeezed = squeeze_context(index, query, options.budget_tokens, options.limit);
    let mut pages_by_key = BTreeMap::new();

    for item in &strict.evidence {
        let page = proof_page_from_strict(item, &strict.coverage);
        pages_by_key.entry(page_key(&page)).or_insert(page);
    }
    for item in &squeezed.evidence {
        let page = proof_page_from_squeezed(item);
        pages_by_key.entry(page_key(&page)).or_insert(page);
    }

    let mut proof_pages = pages_by_key.into_values().collect::<Vec<_>>();
    proof_pages.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.uri.cmp(&b.uri))
    });
    proof_pages.truncate(options.limit);

    let proof_tokens = proof_pages
        .iter()
        .map(|page| page.excerpt_tokens + 20)
        .sum::<u64>();
    let used_tokens = estimate_tokens(query) + proof_tokens;
    let indexed_tokens = index.total_tokens();
    let avoided_context_tokens = indexed_tokens.saturating_sub(used_tokens.min(indexed_tokens));
    let context_reduction_x = indexed_tokens.max(1) as f64 / used_tokens.max(1) as f64;
    let status = fault_status(&strict.coverage, &strict.missing_terms, &proof_pages);
    let fault_id = fault_id(&resolved_handle, query, &proof_pages);

    ContextFaultReport {
        schema: "qorx.context-fault.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        fault_id,
        handle: resolved_handle,
        carrier,
        query: query.to_string(),
        status,
        authorized: true,
        authorization: authorization.reason,
        resolver: "strict-answer+squeeze".to_string(),
        local_only: true,
        provider_calls: 0,
        budget_tokens: options.budget_tokens,
        indexed_tokens,
        used_tokens,
        avoided_context_tokens,
        context_reduction_x,
        proof_pages,
        missing_terms: strict.missing_terms,
        boundary: "A context fault returns small cited proof pages from the local Qorx index. It cannot reveal data that was not indexed or authorized by the handle.".to_string(),
    }
}

pub fn build_context_injection(
    index: &RepoIndex,
    objective: &str,
    options: ContextVmOptions,
) -> ContextInjectReport {
    let options = options.normalized();
    let session = build_session_pointer(index);
    let contract = tool_contract(&session.handle, options.budget_tokens, options.limit);
    let additional_context = prompt_block(&session, objective, &contract, options.budget_tokens);

    ContextInjectReport {
        schema: "qorx.context-inject.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        objective: objective.to_string(),
        handle: session.handle,
        gateway: contract.gateway,
        vm_endpoint: contract.vm_endpoint,
        fault_endpoint: contract.fault_endpoint,
        additional_context,
        local_only: true,
        provider_calls: 0,
        budget_tokens: options.budget_tokens,
        boundary: "Context injection is a compact agent contract. It carries a handle and resolver path, not raw local files.".to_string(),
    }
}

pub fn build_context_nano(
    index: &RepoIndex,
    objective: &str,
    options: ContextVmOptions,
) -> ContextNanoReport {
    let options = options.normalized();
    let session = build_session_pointer(index);
    let carrier = nano_carrier(&session);
    let visible_tokens = estimate_tokens(&carrier).max(1);
    let indexed_tokens = index.total_tokens();
    let avoided_context_tokens = indexed_tokens.saturating_sub(visible_tokens.min(indexed_tokens));
    let context_reduction_x = indexed_tokens.max(1) as f64 / visible_tokens.max(1) as f64;

    ContextNanoReport {
        schema: "qorx.context-nano.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        objective: objective.to_string(),
        carrier,
        handle: session.handle,
        visible_tokens,
        indexed_tokens,
        avoided_context_tokens,
        context_reduction_x,
        local_only: true,
        provider_calls: 0,
        fault_endpoint: format!("{LOCAL_BASE}/context/fault"),
        expand_endpoint: format!("{LOCAL_BASE}/context/expand"),
        budget_tokens: options.budget_tokens,
        boundary: "A Qorx nano carrier is a pointer to local Cosmos context. Cosmos means local Qorx state, not astrophysics. The carrier does not contain the local context; Qorx must expand or fault proof pages locally before any quality claim is made.".to_string(),
    }
}

pub fn build_context_quetta(
    index: &RepoIndex,
    objective: &str,
    options: ContextVmOptions,
) -> ContextQuettaReport {
    let options = options.normalized();
    let session = build_session_pointer(index);
    let visible_tokens = estimate_tokens(QUETTA_ALIAS).max(1);
    let manifest = quetta_manifest(index, &session);
    let value_ledger = quetta_value_ledger();

    ContextQuettaReport {
        schema: "qorx.context-quetta.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        objective: objective.to_string(),
        carrier: QUETTA_ALIAS.to_string(),
        handle: session.handle,
        visible_tokens,
        local_indexed_tokens: index.total_tokens(),
        manifest,
        value_ledger,
        local_only: true,
        provider_calls: 0,
        fault_endpoint: format!("{LOCAL_BASE}/context/fault"),
        expand_endpoint: format!("{LOCAL_BASE}/context/expand"),
        budget_tokens: options.budget_tokens,
        boundary: "Q opens the vault: the Quetta alias is one visible resolver alias for measured local Cosmos state. It contains no hidden context, it is not physical storage proof, and it is not a provider billing record.".to_string(),
    }
}

pub fn expand_nano_carrier(
    index: &RepoIndex,
    carrier: &str,
    options: ContextVmOptions,
) -> ContextExpandReport {
    let options = options.normalized();
    let authorization = authorize_handle(index, carrier);
    let session = build_session_pointer(index);
    let manifest = if authorization.authorized && authorization.reason == "active-quetta-alias" {
        Some(quetta_manifest(index, &session))
    } else {
        None
    };
    let handle = if authorization.authorized {
        authorization.resolved_handle.clone()
    } else {
        String::new()
    };
    let contract = tool_contract(
        if handle.is_empty() {
            &session.handle
        } else {
            &handle
        },
        options.budget_tokens,
        options.limit,
    );
    let additional_context = if authorization.authorized {
        prompt_block(
            &session,
            "expanded nano carrier",
            &contract,
            options.budget_tokens,
        )
    } else {
        String::new()
    };

    ContextExpandReport {
        schema: "qorx.context-expand.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        carrier: carrier.to_string(),
        handle,
        status: if authorization.authorized {
            "resolved".to_string()
        } else {
            "unauthorized".to_string()
        },
        authorized: authorization.authorized,
        authorization: authorization.reason,
        contract,
        manifest,
        additional_context,
        local_only: true,
        provider_calls: 0,
        boundary: authorization.boundary,
    }
}

fn ledger(indexed_tokens: u64, visible_frame_tokens: u64, proof_tokens: u64) -> VmLedger {
    let sent_tokens = visible_frame_tokens + proof_tokens;
    let avoided_context_tokens = indexed_tokens.saturating_sub(sent_tokens.min(indexed_tokens));
    let context_reduction_x = indexed_tokens.max(1) as f64 / sent_tokens.max(1) as f64;
    let pricing = Pricing::from_env();
    let estimated_usd_saved = pricing.input_usd(avoided_context_tokens);
    VmLedger {
        indexed_tokens,
        visible_frame_tokens,
        proof_tokens,
        sent_tokens,
        avoided_context_tokens,
        context_reduction_x,
        estimated_usd_saved,
        pricing,
        estimator: TOKEN_ESTIMATOR_LABEL.to_string(),
        boundary: "Ledger values are deterministic local estimates. They become provider billing claims only when routed provider usage reports or invoices confirm them.".to_string(),
    }
}

fn tool_contract(handle: &str, budget_tokens: u64, limit: usize) -> VmToolContract {
    VmToolContract {
        gateway: LOCAL_BASE.to_string(),
        vm_endpoint: format!("{LOCAL_BASE}/context/vm"),
        fault_endpoint: format!("{LOCAL_BASE}/context/fault"),
        request_shape: serde_json::json!({
            "handle": handle,
            "query": "<specific local evidence question>",
            "budget_tokens": budget_tokens,
            "limit": limit
        }),
        prompt_policy: "Carry the QORX_CONTEXT_VM block. Do not paste broad local context upstream; request proof pages only when local evidence is needed.".to_string(),
        subagent_policy: "Subagents use the same handle and the same /context/fault endpoint instead of receiving raw repo or capsule dumps.".to_string(),
        unsupported_policy: "If Qorx returns not_found or unauthorized, the agent must say the local index does not support the claim.".to_string(),
        billing_policy: "Treat qshf/B2C as local accounting until routed provider usage confirms invoice savings.".to_string(),
        boundary: "This contract gives agents an on-demand local resolver. It does not make outside models understand hidden data without a Qorx call.".to_string(),
    }
}

fn prompt_block(
    session: &SessionPointer,
    objective: &str,
    contract: &VmToolContract,
    budget_tokens: u64,
) -> String {
    format!(
        "QORX_CONTEXT_VM {version} {handle}\nContext VM: active local Cosmos resolver, not a Linux VM.\nGateway: {gateway}\nFault endpoint: {fault_endpoint}\nFault body: {{\"handle\":\"{handle}\",\"query\":\"<specific local evidence question>\",\"budget_tokens\":{budget_tokens}}}\nObjective hint: {objective}\nRule: keep broad local context out of the prompt; ask Qorx for cited proof pages when needed.\nsubagents: use the same handle and fault endpoint; do not receive raw repo dumps.\nBoundary: unsupported or unauthorized Qorx faults must be treated as no local proof.",
        version = CONTEXT_VM_VERSION,
        handle = session.handle,
        gateway = contract.gateway,
        fault_endpoint = contract.fault_endpoint,
    )
}

#[derive(Debug, Clone)]
struct HandleAuthorization {
    authorized: bool,
    reason: String,
    boundary: String,
    resolved_handle: String,
    carrier: Option<String>,
}

fn authorize_handle(index: &RepoIndex, handle: &str) -> HandleAuthorization {
    let session = build_session_pointer(index);
    let active = session.handle.clone();
    let active_carrier = nano_carrier(&session);
    if handle == QUETTA_ALIAS {
        return HandleAuthorization {
            authorized: true,
            reason: "active-quetta-alias".to_string(),
            boundary: "Q matches the active local Qorx quetta resolver alias.".to_string(),
            resolved_handle: active,
            carrier: Some(QUETTA_ALIAS.to_string()),
        };
    }
    if handle == active {
        return HandleAuthorization {
            authorized: true,
            reason: "active-session".to_string(),
            boundary: "Handle matches the active local Qorx session.".to_string(),
            resolved_handle: active,
            carrier: None,
        };
    }
    if handle.starts_with("qfx:") {
        if handle == active_carrier {
            return HandleAuthorization {
                authorized: true,
                reason: "active-nano-carrier".to_string(),
                boundary: "Nano carrier matches the active local Qorx session.".to_string(),
                resolved_handle: active,
                carrier: Some(handle.to_string()),
            };
        }
        return HandleAuthorization {
            authorized: false,
            reason: "stale-nano-carrier".to_string(),
            boundary: "Context faults require the active qfx nano carrier for this local index; stale nano carriers are refused.".to_string(),
            resolved_handle: handle.to_string(),
            carrier: Some(handle.to_string()),
        };
    }
    if !handle.starts_with("qorx://s/") {
        return HandleAuthorization {
            authorized: false,
            reason: "invalid-handle".to_string(),
            boundary:
                "Context faults require the active qorx://s session handle for this local index."
                    .to_string(),
            resolved_handle: handle.to_string(),
            carrier: None,
        };
    }
    HandleAuthorization {
        authorized: false,
        reason: "stale-session".to_string(),
        boundary: "Context faults require the active qorx://s session handle for this local index; stale session handles are refused.".to_string(),
        resolved_handle: handle.to_string(),
        carrier: None,
    }
}

fn nano_carrier(session: &SessionPointer) -> String {
    let suffix = session
        .handle
        .strip_prefix("qorx://s/")
        .unwrap_or(session.handle.as_str());
    let short = &suffix[..suffix.len().min(8)];
    format!("qfx:{short}")
}

fn quetta_manifest(index: &RepoIndex, session: &SessionPointer) -> QuettaManifest {
    QuettaManifest {
        schema: "qorx.quetta-manifest.v1".to_string(),
        alias: QUETTA_ALIAS.to_string(),
        handle: session.handle.clone(),
        root_fingerprint: session.root_fingerprint.clone(),
        manifest_hash: quetta_manifest_hash(index, session),
        quark_count: index.atoms.len(),
        indexed_tokens: index.total_tokens(),
        logical_context_bytes: SYNTHETIC_CONTEXT_BYTES.to_string(),
        logical_context_unit: "quetta-scale-counterfactual-10^30-bytes".to_string(),
        physical_manifest_present: false,
        token_estimator: TOKEN_ESTIMATOR_LABEL.to_string(),
        proof_mode: "sha256-manifest+active-session+context-fault".to_string(),
        lossless_resolver: false,
        boundary: "The active Q alias resolves the measured local index only. Quetta-scale fields are vocabulary labels and counterfactual placeholders until a real chunk store and full hydrate proof are attached.".to_string(),
    }
}

fn quetta_manifest_hash(index: &RepoIndex, session: &SessionPointer) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CONTEXT_VM_VERSION.as_bytes());
    hasher.update(QUETTA_ALIAS.as_bytes());
    hasher.update(SYNTHETIC_CONTEXT_BYTES.as_bytes());
    hasher.update(session.root_fingerprint.as_bytes());
    hasher.update(index.root.as_bytes());
    hasher.update(index.updated_at.to_rfc3339().as_bytes());
    for atom in &index.atoms {
        hasher.update(atom.id.as_bytes());
        hasher.update(atom.hash.as_bytes());
        hasher.update(atom.path.as_bytes());
        hasher.update(atom.start_line.to_le_bytes());
        hasher.update(atom.end_line.to_le_bytes());
        hasher.update(atom.token_estimate.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn quetta_value_ledger() -> QuettaValueLedger {
    QuettaValueLedger {
        schema: "qorx.quetta-value-ledger.v1".to_string(),
        counterfactual_value_usd: COUNTERFACTUAL_VALUE_USD.to_string(),
        visible_alias_cost_usd: VISIBLE_ALIAS_COST_USD.to_string(),
        effective_leverage_x: COUNTERFACTUAL_LEVERAGE_X.to_string(),
        billing_claim: false,
        accounting_mode: "counterfactual-equivalent-not-invoice".to_string(),
        boundary: "The value fields are counterfactual labels for locally owned context. They are not provider invoices, guaranteed cash savings, or physics claims.".to_string(),
    }
}

fn unauthorized_fault(
    index: &RepoIndex,
    handle: &str,
    query: &str,
    options: ContextVmOptions,
    authorization: HandleAuthorization,
) -> ContextFaultReport {
    let indexed_tokens = index.total_tokens();
    let used_tokens = estimate_tokens(query).max(1);
    let avoided_context_tokens = indexed_tokens.saturating_sub(used_tokens.min(indexed_tokens));
    ContextFaultReport {
        schema: "qorx.context-fault.v1".to_string(),
        version: CONTEXT_VM_VERSION.to_string(),
        fault_id: fault_id(handle, query, &[]),
        handle: handle.to_string(),
        carrier: authorization.carrier,
        query: query.to_string(),
        status: "unauthorized".to_string(),
        authorized: false,
        authorization: authorization.reason,
        resolver: "strict-answer+squeeze".to_string(),
        local_only: true,
        provider_calls: 0,
        budget_tokens: options.budget_tokens,
        indexed_tokens,
        used_tokens,
        avoided_context_tokens,
        context_reduction_x: indexed_tokens.max(1) as f64 / used_tokens.max(1) as f64,
        proof_pages: Vec::new(),
        missing_terms: Vec::new(),
        boundary: authorization.boundary,
    }
}

fn capabilities(handle: &str, budget_tokens: u64) -> Vec<VmCapability> {
    [
        ("session", "resolve the active Qorx session pointer"),
        (
            "strict-answer",
            "extract only directly supported indexed evidence",
        ),
        ("squeeze", "return query-relevant lines from ranked quarks"),
        ("pack", "materialize a bounded local working set"),
        ("map", "expand through local symbol and path edges"),
        (
            "context-fault",
            "resolve cited proof pages through the active handle",
        ),
    ]
    .into_iter()
    .map(|(kind, boundary)| VmCapability {
        handle: handle.to_string(),
        kind: kind.to_string(),
        permissions: vec![
            "read_index".to_string(),
            "return_cited_evidence".to_string(),
        ],
        resolver: format!("qorx.context.{kind}"),
        budget_tokens,
        ttl: "local-session".to_string(),
        boundary: boundary.to_string(),
    })
    .collect()
}

fn proof_page_from_strict(item: &StrictEvidence, coverage: &str) -> ProofPage {
    proof_page(ProofPageInput {
        source_kind: "strict-answer",
        quark_id: &item.id,
        path: &item.path,
        start_line: item.start_line,
        end_line: item.end_line,
        excerpt_hash: &item.excerpt_hash,
        support: coverage,
        matched_terms: &item.matched_terms,
        excerpt: &item.excerpt,
    })
}

fn proof_page_from_squeezed(item: &SqueezedEvidence) -> ProofPage {
    proof_page(ProofPageInput {
        source_kind: "squeeze",
        quark_id: &item.id,
        path: &item.path,
        start_line: item.start_line,
        end_line: item.end_line,
        excerpt_hash: &item.excerpt_hash,
        support: "evidence",
        matched_terms: &item.matched_terms,
        excerpt: &item.excerpt,
    })
}

struct ProofPageInput<'a> {
    source_kind: &'a str,
    quark_id: &'a str,
    path: &'a str,
    start_line: usize,
    end_line: usize,
    excerpt_hash: &'a str,
    support: &'a str,
    matched_terms: &'a [String],
    excerpt: &'a str,
}

fn proof_page(input: ProofPageInput<'_>) -> ProofPage {
    let content_hash = content_hash(input.excerpt);
    let uri = format!("qorx://p/{}", &content_hash[..16]);
    ProofPage {
        uri,
        source_kind: input.source_kind.to_string(),
        quark_id: input.quark_id.to_string(),
        path: input.path.to_string(),
        start_line: input.start_line,
        end_line: input.end_line,
        excerpt_hash: input.excerpt_hash.to_string(),
        content_hash,
        support: input.support.to_string(),
        matched_terms: stable_terms(input.matched_terms),
        excerpt_tokens: estimate_tokens(input.excerpt),
        excerpt: input.excerpt.to_string(),
        resolver: format!("qorx.{}", input.source_kind),
    }
}

fn stable_terms(terms: &[String]) -> Vec<String> {
    let set = terms.iter().cloned().collect::<BTreeSet<_>>();
    set.into_iter().collect()
}

fn page_key(page: &ProofPage) -> String {
    format!("{}:{}", page.quark_id, page.excerpt_hash)
}

fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn fault_id(handle: &str, query: &str, pages: &[ProofPage]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(handle.as_bytes());
    hasher.update(query.as_bytes());
    for page in pages {
        hasher.update(page.uri.as_bytes());
        hasher.update(page.content_hash.as_bytes());
    }
    format!("qvf_{}", &format!("{:x}", hasher.finalize())[..12])
}

fn fault_status(coverage: &str, missing_terms: &[String], pages: &[ProofPage]) -> String {
    if pages.is_empty() {
        return "not_found".to_string();
    }
    if coverage == "supported" && missing_terms.is_empty() {
        return "resolved".to_string();
    }
    if coverage == "partial" || !missing_terms.is_empty() {
        return "partial".to_string();
    }
    "resolved".to_string()
}
