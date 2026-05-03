use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    env::temp_dir().join(format!("qorx-context-protobuf-{suffix}"))
}

fn seed_context(dir: &Path) {
    fs::create_dir_all(dir).expect("create qorx home");
    fs::write(
        dir.join("repo_index.json"),
        r#"{
  "root": "C:/repo",
  "updated_at": "2026-04-29T00:00:00Z",
  "quarks": [{
    "id": "qva_demo",
    "path": "src/demo.rs",
    "start_line": 1,
    "end_line": 3,
    "hash": "abc",
    "token_estimate": 12,
    "symbols": ["demo"],
    "signal_mask": 4,
    "vector": [1, 2, 3],
    "text": "pub fn demo() {}"
  }]
}"#,
    )
    .expect("write index");
    fs::write(
        dir.join("quarks.json"),
        r#"{"quarks":{"qvk_demo":"pub fn demo() {}"}}"#,
    )
    .expect("write quark store");
    fs::write(dir.join("stats.json"), r#"{"requests":0}"#).expect("write stats");
    fs::write(dir.join("response_cache.json"), r#"{"entries":{}}"#).expect("write cache");
    fs::write(dir.join("integrations.json"), r#"{"targets":[]}"#).expect("write integrations");
    fs::write(
        dir.join("qorx-provenance.json"),
        r#"{"schema":"qorx.hybrid-provenance.v1","verified":true}"#,
    )
    .expect("write provenance");
}

#[test]
fn context_snapshot_saves_and_verifies_full_local_context_as_protobuf() {
    let qorx_home = unique_temp_dir();
    seed_context(&qorx_home);

    let snapshot = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["context", "snapshot"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx context snapshot");

    assert!(
        snapshot.status.success(),
        "context snapshot failed: status={:?} stderr={}",
        snapshot.status.code(),
        String::from_utf8_lossy(&snapshot.stderr)
    );
    assert!(qorx_home.join("qorx-context.pb").exists());
    for file_name in [
        "repo_index.pb",
        "quarks.pb",
        "stats.pb",
        "response_cache.pb",
        "integrations.pb",
        "qorx-provenance.pb",
        "qorx-context.pb",
    ] {
        let bytes = fs::read(qorx_home.join(file_name)).expect("read protobuf state file");
        assert_ne!(
            bytes.first().copied(),
            Some(b'{'),
            "{file_name} should be protobuf-envelope state, not raw JSON"
        );
    }

    let snapshot_report: serde_json::Value =
        serde_json::from_slice(&snapshot.stdout).expect("parse snapshot stdout as json");
    assert_eq!(snapshot_report["verified"], true);
    assert_eq!(snapshot_report["coverage_percent"], 100.0);
    assert_eq!(snapshot_report["index_quarks"], 1);
    assert_eq!(snapshot_report["quark_store_entries"], 1);

    let verify = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["context", "verify"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx context verify");

    assert!(
        verify.status.success(),
        "context verify failed: status={:?} stderr={}",
        verify.status.code(),
        String::from_utf8_lossy(&verify.stderr)
    );
    let verify_report: serde_json::Value =
        serde_json::from_slice(&verify.stdout).expect("parse verify stdout as json");
    assert_eq!(verify_report["verified"], true);
    assert_eq!(verify_report["coverage_percent"], 100.0);
    assert_eq!(verify_report["files_checked"], 10);

    let _ = fs::remove_dir_all(&qorx_home);
}
