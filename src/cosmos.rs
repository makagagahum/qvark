use std::path::PathBuf;

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{config::AppPaths, lexicon, proto_store, qorx::QorxRunReport};

const COSMOS_FILE: &str = "qorx-cosmos.pb";
const COSMOS_BOUNDARY: &str = "The Qorx cosmos is local protobuf-envelope state. It can store large local traces behind tiny handles, but it does not bypass provider billing or physical information limits.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosLedger {
    pub schema: String,
    pub created_at: String,
    pub updated_at: String,
    pub events: Vec<CosmosEvent>,
    pub vocabulary: Value,
    pub boundary: String,
}

impl Default for CosmosLedger {
    fn default() -> Self {
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        Self {
            schema: "qorx.cosmos.v1".to_string(),
            created_at: now.clone(),
            updated_at: now,
            events: Vec::new(),
            vocabulary: lexicon::vocabulary(),
            boundary: COSMOS_BOUNDARY.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosEvent {
    pub event_id: String,
    pub handle: String,
    pub event_kind: String,
    pub created_at: String,
    pub carrier: String,
    pub matter: String,
    pub source: String,
    pub file: String,
    pub mode: String,
    pub goal_hash: String,
    pub visible_tokens: u64,
    pub provider_calls: u64,
    pub artifact_schema: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosReceipt {
    pub schema: String,
    pub handle: String,
    pub event_id: String,
    pub event_kind: String,
    pub storage: String,
    pub event_count: usize,
    pub carrier: String,
    pub matter: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CosmosStatus {
    pub schema: String,
    pub storage: String,
    pub event_count: usize,
    pub events: Vec<CosmosEvent>,
    pub vocabulary: Value,
    pub boundary: String,
}

pub fn path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(COSMOS_FILE)
}

pub fn status(paths: &AppPaths) -> Result<CosmosStatus> {
    let ledger = load(paths)?;
    Ok(CosmosStatus {
        schema: ledger.schema,
        storage: path(paths).display().to_string(),
        event_count: ledger.events.len(),
        events: ledger.events,
        vocabulary: ledger.vocabulary,
        boundary: ledger.boundary,
    })
}

pub fn record_run(
    paths: &AppPaths,
    event_kind: &str,
    run: &QorxRunReport,
) -> Result<CosmosReceipt> {
    let mut ledger = load(paths)?;
    let payload = serde_json::to_value(run)?;
    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let seed = serde_json::to_vec(&json!({
        "event_kind": event_kind,
        "created_at": created_at,
        "file": run.file,
        "mode": run.program.mode,
        "goal": run.program.goal,
        "visible_tokens": run.visible_tokens,
        "provider_calls": run.provider_calls,
        "payload": payload,
    }))?;
    let event_hash = hex_sha256(&seed);
    let short = short_hash(&event_hash);
    let event = CosmosEvent {
        event_id: format!("qorx-event-{short}"),
        handle: format!("qorx://u/{short}"),
        event_kind: event_kind.to_string(),
        created_at,
        carrier: "photon".to_string(),
        matter: "local_action_trace".to_string(),
        source: if run.source_kind == "qorxb" {
            "collapsed_bytecode".to_string()
        } else {
            "wavefunction".to_string()
        },
        file: run.file.clone(),
        mode: run.program.mode.clone(),
        goal_hash: hex_sha256(run.program.goal.as_bytes()),
        visible_tokens: run.visible_tokens,
        provider_calls: run.provider_calls,
        artifact_schema: run.schema.clone(),
        payload,
    };
    ledger.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    ledger.events.push(event.clone());
    proto_store::save(&path(paths), &ledger)?;

    Ok(CosmosReceipt {
        schema: "qorx.cosmos.receipt.v1".to_string(),
        handle: event.handle,
        event_id: event.event_id,
        event_kind: event.event_kind,
        storage: path(paths).display().to_string(),
        event_count: ledger.events.len(),
        carrier: event.carrier,
        matter: event.matter,
        boundary: "The large run/action trace is in the local cosmos ledger; the outside model-visible carrier is only a photon handle/report.".to_string(),
    })
}

fn load(paths: &AppPaths) -> Result<CosmosLedger> {
    proto_store::load_or_default(&path(paths), &[])
}

fn short_hash(hash: &str) -> &str {
    hash.get(..16).unwrap_or(hash)
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
