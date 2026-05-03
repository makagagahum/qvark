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
    env::temp_dir().join(format!("qorx-stats-reset-{suffix}"))
}

fn seed_stats_file(dir: &Path) {
    fs::create_dir_all(dir).expect("create qorx home");
    let stats = r#"{
  "started_at": "2026-04-29T00:00:00Z",
  "updated_at": "2026-04-29T00:00:00Z",
  "requests": 9,
  "raw_prompt_tokens": 5000,
  "compressed_prompt_tokens": 1200,
  "saved_prompt_tokens": 3800,
  "upstream_errors": 2,
  "quarks_created": 11,
  "cache_hits": 4,
  "cache_saved_prompt_tokens": 2500,
  "provider_cached_prompt_tokens": 3200,
  "provider_cache_write_tokens": 800,
  "context_pack_requests": 7,
  "context_indexed_tokens": 10000,
  "context_sent_tokens": 850,
  "context_omitted_tokens": 9150,
  "last_provider": "openai"
}"#;
    fs::write(dir.join("stats.json"), stats).expect("write seeded stats");
}

fn seed_minimal_index(dir: &Path) {
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
    fs::write(dir.join("stats.json"), r#"{"requests":0}"#).expect("write stats");
}

#[test]
fn stats_reset_clears_existing_counters() {
    let qorx_home = unique_temp_dir();
    seed_stats_file(&qorx_home);

    let output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["stats", "reset"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx stats reset");

    assert!(
        output.status.success(),
        "stats reset failed: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let printed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse reset stdout as json");
    assert_eq!(printed["requests"], 0);
    assert_eq!(printed["cache_hits"], 0);
    assert_eq!(printed["context_pack_requests"], 0);
    assert!(printed["last_provider"].is_null());

    let stats_pb = qorx_home.join("stats.pb");
    assert!(
        stats_pb.exists(),
        "stats reset should persist protobuf state"
    );
    assert_ne!(
        fs::read(&stats_pb)
            .expect("read stats protobuf")
            .first()
            .copied(),
        Some(b'{'),
        "stats protobuf should not be a JSON file"
    );

    let stats_output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["stats"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx stats");
    assert!(stats_output.status.success());
    let persisted: serde_json::Value =
        serde_json::from_slice(&stats_output.stdout).expect("parse persisted stats");
    assert_eq!(persisted["requests"], 0);
    assert_eq!(persisted["saved_prompt_tokens"], 0);
    assert_eq!(persisted["context_omitted_tokens"], 0);
    assert!(persisted["last_provider"].is_null());

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn session_pointer_does_not_increment_usage_stats() {
    let qorx_home = unique_temp_dir();
    seed_minimal_index(&qorx_home);

    let session = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["session"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx session");
    assert!(
        session.status.success(),
        "session failed: status={:?} stderr={}",
        session.status.code(),
        String::from_utf8_lossy(&session.stderr)
    );

    let stats_output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["stats"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx stats");
    assert!(stats_output.status.success());
    let stats: serde_json::Value =
        serde_json::from_slice(&stats_output.stdout).expect("parse stats");
    assert_eq!(stats["context_pack_requests"], 0);
    assert_eq!(stats["context_indexed_tokens"], 0);
    assert_eq!(stats["context_sent_tokens"], 0);

    let _ = fs::remove_dir_all(&qorx_home);
}
