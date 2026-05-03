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
        "qorx-cosmos-{}-{suffix}-{sequence}",
        std::process::id()
    ))
}

fn seed_index(dir: &Path) {
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
fn qorx_run_writes_cosmos_event_to_local_protobuf_ledger() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_index(&qorx_home);
    let qorx_file = root.join("strict.qorx");
    fs::write(
        &qorx_file,
        "mode: strict-answer\nask: production gate routed provider evidence\nlimit: 1\n",
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["lexicon"]["source"], "qwav");
    assert_eq!(report["lexicon"]["model_visible_carrier"], "phot");
    assert_eq!(report["lexicon"]["local_runtime"], "qosm");
    assert_eq!(report["lexicon"]["cost_transform"], "qshf");
    assert_eq!(report["cosmos"]["schema"], "qorx.cosmos.receipt.v1");
    assert!(report["cosmos"]["handle"]
        .as_str()
        .unwrap()
        .starts_with("qorx://u/"));
    assert_eq!(report["cosmos"]["event_kind"], "qorx.run");
    assert_eq!(report["cosmos"]["event_count"], 1);
    assert!(report["cosmos"]["storage"]
        .as_str()
        .unwrap()
        .ends_with("qorx-cosmos.pb"));

    let ledger = qorx_home.join("qorx-cosmos.pb");
    assert!(ledger.exists(), "cosmos ledger should be stored locally");
    assert_ne!(
        fs::read(&ledger)
            .expect("read cosmos protobuf")
            .first()
            .copied(),
        Some(b'{'),
        "cosmos ledger must be protobuf-envelope state, not raw JSON"
    );

    let status = qorx(&["cosmos", "status"], &qorx_home);
    assert_eq!(status["schema"], "qorx.cosmos.v1");
    assert_eq!(status["event_count"], 1);
    assert_eq!(status["events"][0]["event_kind"], "qorx.run");
    assert_eq!(status["events"][0]["carrier"], "photon");
    assert_eq!(status["events"][0]["matter"], "local_action_trace");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn lexicon_report_names_qorx_ai_language_vocabulary() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_index(&qorx_home);

    let report = qorx(&["lexicon"], &qorx_home);

    assert_eq!(report["schema"], "qorx.lexicon.v1");
    assert_eq!(report["language"], "qorx");
    assert_eq!(report["format"], "protobuf-envelope");
    assert_eq!(report["vocabulary"][".qorx"], "qwav_source");
    assert_eq!(report["vocabulary"][".qorxb"], "qfal_bytecode");
    assert_eq!(report["vocabulary"]["qorx://u"], "qsng_handle");
    assert_eq!(report["vocabulary"]["qosm"], "local_resolver_ledger");
    assert_eq!(
        report["vocabulary"]["qshf"],
        "baseline_to_compact_accounting"
    );
    assert_eq!(
        report["vocabulary"]["cosmos_store"],
        "qorx_data_dir_or_portable_store"
    );
    assert_eq!(report["vocabulary"]["b2c"], "qshf_accounting");
    assert_eq!(
        report["vocabulary"]["cost_transform"],
        "qshf_compaction_transform"
    );
    assert_eq!(
        report["vocabulary"]["qv0d"],
        "resolver_miss_or_empty_evidence"
    );
    assert_eq!(report["aliases"]["qvoid"], "qv0d");
    let terms = report["terms"].as_array().expect("terms");
    assert!(terms.len() >= 50);
    assert!(
        terms
            .iter()
            .all(|term| (3..=4).contains(&term["name"].as_str().unwrap().len())),
        "primary terms must stay 3-4 chars"
    );
    assert!(terms
        .iter()
        .any(|term| term["name"] == "qv0d" && term["kind"] == "runtime"));
    assert!(report["layers"]["cost_transform"]
        .as_str()
        .unwrap()
        .contains("local counters"));
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("googolplex"),
        "serious lexicon should not expose googolplex wording"
    );
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("not a physics engine"));
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("provider billing is not bypassed"));

    let _ = fs::remove_dir_all(&root);
}
