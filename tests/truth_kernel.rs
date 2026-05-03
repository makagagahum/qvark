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
    env::temp_dir().join(format!("qorx-truth-kernel-{suffix}"))
}

fn seed_truth_index(dir: &Path) {
    fs::create_dir_all(dir).expect("create qorx home");
    fs::write(
        dir.join("repo_index.json"),
        r#"{
  "root": "C:/repo",
  "updated_at": "2026-04-29T00:00:00Z",
  "quarks": [
    {
      "id": "qva_money",
      "path": "src/money.rs",
      "start_line": 81,
      "end_line": 90,
      "hash": "abc",
      "token_estimate": 28,
      "symbols": ["production_gate_passed", "routed_provider_requests"],
      "signal_mask": 4,
      "vector": [11, 12, 13],
      "text": "production gate requires routed provider savings evidence before money claims are allowed"
    },
    {
      "id": "qva_agent",
      "path": "src/agent.rs",
      "start_line": 1,
      "end_line": 8,
      "hash": "def",
      "token_estimate": 24,
      "symbols": ["deterministic_subatomic_agent"],
      "signal_mask": 64,
      "vector": [21, 22, 23],
      "text": "the qorx agent is deterministic and local only; it may plan session strict answer and pack steps"
    },
    {
      "id": "qva_fixture",
      "path": "tests/truth_kernel.rs",
      "start_line": 40,
      "end_line": 44,
      "hash": "ghi",
      "token_estimate": 14,
      "symbols": ["strict_answer_fixture"],
      "signal_mask": 4,
      "vector": [31, 32, 33],
      "text": ".args([\"strict-answer\", \"warp drive cooking schedule\"])"
    },
    {
      "id": "qva_doc_one",
      "path": "docs/proof-one.md",
      "start_line": 1,
      "end_line": 3,
      "hash": "jkl",
      "token_estimate": 12,
      "symbols": [],
      "signal_mask": 0,
      "vector": [41, 42, 43],
      "text": "production gate provider savings are documented in this secondary note"
    },
    {
      "id": "qva_doc_two",
      "path": "docs/proof-two.md",
      "start_line": 1,
      "end_line": 3,
      "hash": "mno",
      "token_estimate": 12,
      "symbols": [],
      "signal_mask": 0,
      "vector": [51, 52, 53],
      "text": "provider savings evidence can be repeated by docs but agent output stays small"
    }
  ]
}"#,
    )
    .expect("write index");
    fs::write(dir.join("stats.json"), r#"{"requests":0}"#).expect("write stats");
}

#[test]
fn strict_answer_cites_quarks_and_refuses_unindexed_claims() {
    let qorx_home = unique_temp_dir();
    seed_truth_index(&qorx_home);

    let supported = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["strict-answer", "production gate routed provider evidence"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx strict-answer");
    assert!(
        supported.status.success(),
        "strict-answer failed: status={:?} stderr={}",
        supported.status.code(),
        String::from_utf8_lossy(&supported.stderr)
    );
    let supported_json: serde_json::Value =
        serde_json::from_slice(&supported.stdout).expect("parse strict answer");
    assert_eq!(supported_json["schema"], "qorx.strict-answer.v1");
    assert_eq!(supported_json["coverage"], "supported");
    assert_eq!(supported_json["evidence"][0]["id"], "qva_money");
    assert!(supported_json["answer"]
        .as_str()
        .unwrap()
        .contains("production gate requires routed provider savings evidence"));

    let missing = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["strict-answer", "warp drive cooking schedule"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx strict-answer missing");
    assert!(missing.status.success());
    let missing_json: serde_json::Value =
        serde_json::from_slice(&missing.stdout).expect("parse missing strict answer");
    assert_eq!(missing_json["coverage"], "not_found");
    assert_eq!(missing_json["answer"], "");
    assert!(missing_json["evidence"].as_array().unwrap().is_empty());

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn agent_executes_subatomic_truth_plan_without_provider_calls() {
    let qorx_home = unique_temp_dir();
    seed_truth_index(&qorx_home);

    let output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args([
            "agent",
            "prove production gate provider savings",
            "--budget-tokens",
            "600",
        ])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx agent");
    assert!(
        output.status.success(),
        "agent failed: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse agent");
    assert_eq!(report["schema"], "qorx.agent.v1");
    assert_eq!(report["agent_name"], "Marvin");
    assert_eq!(report["mode"], "deterministic_subatomic");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert_eq!(
        report["contract"]["hallucination_policy"],
        "refuse_unsupported_indexed_context"
    );
    assert_eq!(
        report["contract"]["compression_policy"],
        "subatomic_context_budget"
    );
    assert_eq!(report["contract"]["b2c_policy"], "account_then_claim");
    assert!(report["contract"]["boundary"]
        .as_str()
        .unwrap()
        .contains("Downstream model correctness"));
    assert_eq!(report["strict_answer"]["coverage"], "supported");
    assert!(
        report["strict_answer"]["evidence"]
            .as_array()
            .unwrap()
            .len()
            <= 2,
        "agent strict evidence should stay subatomic by default"
    );
    assert!(report["packed_context"]["quarks_used"].as_u64().unwrap() > 0);

    let actions = report["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step["action"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(actions, ["session", "strict-answer", "pack"]);

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn marvin_alias_uses_the_same_strict_agent_contract() {
    let qorx_home = unique_temp_dir();
    seed_truth_index(&qorx_home);

    let output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args([
            "marvin",
            "prove production gate provider savings",
            "--budget-tokens",
            "600",
        ])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run qorx marvin");
    assert!(
        output.status.success(),
        "marvin failed: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse marvin");
    assert_eq!(report["agent_name"], "Marvin");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert_eq!(
        report["contract"]["hallucination_policy"],
        "refuse_unsupported_indexed_context"
    );
    assert_eq!(report["contract"]["b2c_policy"], "account_then_claim");

    let _ = fs::remove_dir_all(&qorx_home);
}
