use std::path::Path;

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    config::{self, AppPaths},
    cosmos,
    index::RepoIndex,
    lexicon,
    qorx::{self, QorxRunReport},
};

const A2A_MEDIA_TYPE: &str = "application/a2a+json";
const A2A_BOUNDARY: &str = "A2A shape only; this CLI command is not a long-running network server.";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub protocol_version: String,
    pub name: String,
    pub description: String,
    pub url: String,
    pub preferred_transport: String,
    pub supported_interfaces: Vec<AgentInterface>,
    pub capabilities: AgentCapabilities,
    pub default_input_modes: Vec<String>,
    pub default_output_modes: Vec<String>,
    pub skills: Vec<AgentSkill>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInterface {
    pub protocol_binding: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    pub streaming: bool,
    pub push_notifications: bool,
    pub state_transition_history: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub input_modes: Vec<String>,
    pub output_modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub media_type: String,
    pub task: A2aTask,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aTask {
    pub id: String,
    pub context_id: String,
    pub status: TaskStatus,
    pub history: Vec<Message>,
    pub artifacts: Vec<Artifact>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    pub state: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: String,
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub artifact_id: String,
    pub name: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

pub fn agent_card() -> AgentCard {
    AgentCard {
        protocol_version: "1.0".to_string(),
        name: "Qorx".to_string(),
        description: "A local Qorx resolver and Cosmos ledger for .qorx programs, qorx:// handles, and evidence-bounded execution.".to_string(),
        url: format!("{}/a2a", config::LOCAL_BASE),
        preferred_transport: "HTTP+JSON".to_string(),
        supported_interfaces: vec![AgentInterface {
            protocol_binding: "HTTP+JSON".to_string(),
            url: config::LOCAL_BASE.to_string(),
        }],
        capabilities: AgentCapabilities {
            streaming: false,
            push_notifications: false,
            state_transition_history: true,
        },
        default_input_modes: vec!["text/plain".to_string(), "application/qorx".to_string()],
        default_output_modes: vec!["application/json".to_string(), A2A_MEDIA_TYPE.to_string()],
        skills: vec![
            AgentSkill {
                id: "qorx.resolve".to_string(),
                name: "Resolve Qorx handles".to_string(),
                description: "Resolve .qorx wavefunctions, .qorxb collapsed bytecode, or qorx:// singularity handles through local indexed evidence without exposing bulk local state by default.".to_string(),
                tags: vec!["qorx".to_string(), "cosmos".to_string(), "resolver".to_string()],
                input_modes: vec!["application/qorx".to_string(), "text/plain".to_string()],
                output_modes: vec!["application/json".to_string()],
            },
            AgentSkill {
                id: "qorx.compile".to_string(),
                name: "Compile Qorx bytecode".to_string(),
                description: "Collapse compact .qorx wavefunction source into protobuf-envelope .qorxb bytecode.".to_string(),
                tags: vec!["qorx".to_string(), "collapse".to_string(), "bytecode".to_string()],
                input_modes: vec!["application/qorx".to_string(), "text/plain".to_string()],
                output_modes: vec!["application/json".to_string()],
            },
            AgentSkill {
                id: "qorx.prompt".to_string(),
                name: "Emit Qorx prompt contract".to_string(),
                description: "Emit the photon-sized prompt block and tool contract that tells third-party models to call the Qorx resolver.".to_string(),
                tags: vec!["qorx".to_string(), "photon".to_string(), "tool-contract".to_string()],
                input_modes: vec!["application/qorx".to_string(), "text/plain".to_string()],
                output_modes: vec!["application/json".to_string(), "text/plain".to_string()],
            },
        ],
    }
}

pub fn task_from_file(
    path: &Path,
    index: &RepoIndex,
    paths: Option<&AppPaths>,
) -> Result<TaskResponse> {
    let run = qorx::run_file(path, index)?;
    let mut run_data = serde_json::to_value(&run)?;
    let cosmos = if let Some(paths) = paths {
        Some(cosmos::record_run(paths, "a2a.task", &run)?)
    } else {
        None
    };
    if let Value::Object(map) = &mut run_data {
        map.insert(
            "lexicon".to_string(),
            lexicon::runtime_tags(&run.source_kind),
        );
        if let Some(cosmos) = &cosmos {
            map.insert("cosmos".to_string(), serde_json::to_value(cosmos)?);
        }
    }
    let base = stable_hash(&serde_json::to_vec(&run)?);
    let short = short_hash(&base);
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    Ok(TaskResponse {
        media_type: A2A_MEDIA_TYPE.to_string(),
        task: A2aTask {
            id: format!("qorx-task-{short}"),
            context_id: format!(
                "qorx-context-{}",
                short_hash(&stable_hash(path.display().to_string().as_bytes()))
            ),
            status: TaskStatus {
                state: "TASK_STATE_COMPLETED".to_string(),
                timestamp: timestamp.clone(),
            },
            history: vec![Message {
                message_id: format!("qorx-message-{short}"),
                role: "ROLE_USER".to_string(),
                parts: vec![Part {
                    text: Some(message_text(&run)),
                    data: None,
                    media_type: Some("text/plain".to_string()),
                }],
            }],
            artifacts: vec![Artifact {
                artifact_id: format!("qorx-artifact-{short}"),
                name: "qorx.run".to_string(),
                parts: vec![Part {
                    text: None,
                    data: Some(run_data),
                    media_type: Some("application/json".to_string()),
                }],
            }],
            metadata: json!({
                "localOnly": true,
                "providerCalls": 0,
                "sourceKind": run.source_kind,
                "lexicon": lexicon::runtime_tags(&run.source_kind),
                "cosmos": cosmos,
                "boundary": A2A_BOUNDARY,
            }),
        },
    })
}

fn message_text(run: &QorxRunReport) -> String {
    format!(
        "Run Qorx mode `{}` for goal hash {} from {}.",
        run.program.mode,
        stable_hash(run.program.goal.as_bytes()),
        run.file
    )
}

fn stable_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn short_hash(hash: &str) -> &str {
    hash.get(..16).unwrap_or(hash)
}
