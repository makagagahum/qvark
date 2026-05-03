use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "qorx-a2a-{}-{suffix}-{sequence}",
        std::process::id()
    ))
}

fn seed_qorx_index(dir: &Path) {
    fs::create_dir_all(dir).expect("create qorx home");
    fs::write(
        dir.join("repo_index.json"),
        r#"{
  "root": "C:/repo",
  "updated_at": "2026-04-30T00:00:00Z",
  "qorx": [
    {
      "id": "qorx_money",
      "path": "src/money.rs",
      "start_line": 1,
      "end_line": 3,
      "hash": "abc",
      "token_estimate": 28,
      "symbols": ["production_gate_passed", "routed_provider_requests"],
      "signal_mask": 4,
      "vector": [11, 12, 13],
      "text": "production gate requires routed provider savings evidence before money claims are allowed"
    }
  ]
}"#,
    )
    .expect("write index");
    fs::write(dir.join("stats.json"), r#"{"requests":0}"#).expect("write stats");
}

fn qorx(args: &[&str], qorx_home: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(args)
        .env("QORX_HOME", qorx_home)
        .output()
        .unwrap_or_else(|err| panic!("run qorx {args:?}: {err}"));
    assert!(
        output.status.success(),
        "qorx {args:?} failed: status={:?} stderr={} stdout={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "parse qorx {args:?} JSON: {err}\nstdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn a2a_card_exports_qorx_agent_discovery_shape() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);

    let card = qorx(&["a2a", "card"], &qorx_home);

    assert_eq!(card["name"], "Qorx");
    assert!(card["description"]
        .as_str()
        .unwrap()
        .contains("local Qorx resolver"));
    assert_eq!(card["preferredTransport"], "HTTP+JSON");
    assert_eq!(card["protocolVersion"], "1.0");
    assert_eq!(
        card["supportedInterfaces"][0]["protocolBinding"],
        "HTTP+JSON"
    );
    assert_eq!(card["capabilities"]["streaming"], false);
    assert_eq!(card["capabilities"]["pushNotifications"], false);
    assert!(card["defaultInputModes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|mode| mode == "application/qorx"));
    assert!(card["skills"]
        .as_array()
        .unwrap()
        .iter()
        .any(|skill| skill["id"] == "qorx.resolve"));

    let serialized = serde_json::to_string(&card).expect("serialize card");
    let old_name = ["Q", "vark"].concat();
    let old_slug = ["q", "vark"].concat();
    assert!(!serialized.contains(&old_name));
    assert!(!serialized.contains(&old_slug));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a2a_task_wraps_qorx_run_as_completed_task_with_artifact() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("strict.qorx");
    fs::write(
        &qorx_file,
        "mode: strict-answer\nask: production gate routed provider evidence\nlimit: 1\n",
    )
    .expect("write qorx file");

    let response = qorx(&["a2a", "task", qorx_file.to_str().unwrap()], &qorx_home);
    let task = &response["task"];

    assert_eq!(response["mediaType"], "application/a2a+json");
    assert!(task["id"].as_str().unwrap().starts_with("qorx-task-"));
    assert_eq!(task["status"]["state"], "TASK_STATE_COMPLETED");
    assert_eq!(task["history"][0]["role"], "ROLE_USER");
    assert_eq!(task["artifacts"][0]["name"], "qorx.run");
    assert_eq!(
        task["artifacts"][0]["parts"][0]["mediaType"],
        "application/json"
    );
    assert_eq!(
        task["artifacts"][0]["parts"][0]["data"]["schema"],
        "qorx.run.v1"
    );
    assert_eq!(
        task["artifacts"][0]["parts"][0]["data"]["execution"]["coverage"],
        "supported"
    );
    assert_eq!(
        task["metadata"]["boundary"],
        "A2A shape only; this CLI command is not a long-running network server."
    );
    assert_eq!(task["metadata"]["lexicon"]["local_runtime"], "qosm");
    assert_eq!(task["metadata"]["lexicon"]["cost_transform"], "qshf");
    assert!(task["metadata"]["cosmos"]["handle"]
        .as_str()
        .unwrap()
        .starts_with("qorx://u/"));
    assert_eq!(task["metadata"]["cosmos"]["event_kind"], "a2a.task");
    assert!(qorx_home.join("qorx-cosmos.pb").exists());

    let serialized = serde_json::to_string(&response).expect("serialize task");
    let old_name = ["Q", "vark"].concat();
    assert!(!serialized.contains(&old_name));

    let _ = fs::remove_dir_all(&root);
}
