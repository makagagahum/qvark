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
        "qorx-context-vm-{}-{suffix}-{sequence}",
        std::process::id()
    ))
}

fn seed_index(dir: &Path) {
    fs::create_dir_all(dir).expect("create qorx home");
    fs::write(
        dir.join("repo_index.json"),
        r#"{
  "root": "C:/repo",
  "updated_at": "2026-05-01T00:00:00Z",
  "quarks": [
    {
      "id": "qva_money",
      "path": "src/money.rs",
      "start_line": 81,
      "end_line": 90,
      "hash": "abc",
      "token_estimate": 120,
      "symbols": ["production_gate_passed", "routed_provider_requests"],
      "signal_mask": 0,
      "vector": [11, 12, 13],
      "text": "production gate requires routed provider savings evidence before money claims are allowed\nfiller alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu\nfiller unrelated screen copy should not survive query aware squeezing\nrouted provider requests must be observed before B2C money claims pass"
    },
    {
      "id": "qva_auth_route",
      "path": "src/routes/auth.ts",
      "start_line": 1,
      "end_line": 8,
      "hash": "def",
      "token_estimate": 40,
      "symbols": ["loginRoute"],
      "signal_mask": 66,
      "vector": [21, 22, 23],
      "text": "export function loginRoute(req) {\n  const session = issueSession(req.user);\n  logAudit(session.id);\n  return session;\n}"
    },
    {
      "id": "qva_session_service",
      "path": "src/services/session.ts",
      "start_line": 1,
      "end_line": 5,
      "hash": "ghi",
      "token_estimate": 32,
      "symbols": ["issueSession"],
      "signal_mask": 64,
      "vector": [31, 32, 33],
      "text": "export function issueSession(user) {\n  return { id: user.id, expires: Date.now() + 3600 };\n}"
    },
    {
      "id": "qva_audit_service",
      "path": "src/services/audit.ts",
      "start_line": 1,
      "end_line": 5,
      "hash": "jkl",
      "token_estimate": 28,
      "symbols": ["logAudit"],
      "signal_mask": 64,
      "vector": [41, 42, 43],
      "text": "export function logAudit(sessionId) {\n  return `audit:${sessionId}`;\n}"
    }
  ]
}"#,
    )
    .expect("write index");
}

fn seed_synthetic_scale_index(dir: &Path, token_estimate: u64) {
    fs::create_dir_all(dir).expect("create qorx home");
    fs::write(
        dir.join("repo_index.json"),
        format!(
            r#"{{
  "root": "C:/repo",
  "updated_at": "2026-05-01T00:00:00Z",
  "quarks": [
    {{
      "id": "qva_synthetic_scale",
      "path": "docs/synthetic-scale.md",
      "start_line": 1,
      "end_line": 4,
      "hash": "syntheticscalehash",
      "token_estimate": {token_estimate},
      "symbols": ["SyntheticScale"],
      "signal_mask": 0,
      "vector": [71, 72, 73],
      "text": "synthetic scale sentinel proves Qorx can account huge local token estimates behind Q and still return proof pages"
    }}
  ]
}}"#
        ),
    )
    .expect("write synthetic scale index");
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
fn context_vm_returns_handle_capabilities_fault_and_ledger() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let report = qorx(
        &[
            "context",
            "vm",
            "login route session audit",
            "--budget-tokens",
            "320",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-vm.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["session"]["handle"]
        .as_str()
        .unwrap()
        .starts_with("qorx://s/"));
    assert!(report["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "squeeze"));
    assert_eq!(
        report["contract"]["fault_endpoint"],
        "http://127.0.0.1:47187/context/fault"
    );
    assert!(report["contract"]["subagent_policy"]
        .as_str()
        .unwrap()
        .contains("same handle"));
    assert!(report["prompt_block"]
        .as_str()
        .unwrap()
        .contains("QORX_CONTEXT_VM"));
    assert!(report["prompt_block"]
        .as_str()
        .unwrap()
        .contains("/context/fault"));
    assert!(report["page_faults"][0]["proof_pages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|page| page["path"] == "src/routes/auth.ts"));
    assert!(report["ledger"]["avoided_context_tokens"].as_u64().unwrap() > 0);
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("not a Linux virtual machine"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_fault_returns_small_cited_proof_pages() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let report = qorx(
        &[
            "context",
            "fault",
            "production gate routed provider evidence",
            "--budget-tokens",
            "180",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-fault.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["authorized"], true);
    assert!(matches!(
        report["status"].as_str().unwrap(),
        "resolved" | "partial"
    ));
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["used_tokens"].as_u64().unwrap() <= 180);
    assert!(report["proof_pages"][0]["uri"]
        .as_str()
        .unwrap()
        .starts_with("qorx://p/"));
    assert_eq!(report["proof_pages"][0]["path"], "src/money.rs");
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("local Qorx index"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_fault_rejects_invalid_or_stale_handles() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let report = qorx(
        &[
            "context",
            "fault",
            "production gate routed provider evidence",
            "--handle",
            "qorx://s/stale-or-fake",
            "--budget-tokens",
            "180",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-fault.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["status"], "unauthorized");
    assert_eq!(report["authorized"], false);
    assert!(report["proof_pages"].as_array().unwrap().is_empty());
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("active qorx://s session handle"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_inject_returns_compact_agent_loop_contract() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let report = qorx(
        &[
            "context",
            "inject",
            "login route session audit",
            "--budget-tokens",
            "320",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-inject.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["handle"].as_str().unwrap().starts_with("qorx://s/"));
    assert!(report["additional_context"]
        .as_str()
        .unwrap()
        .contains("Context VM"));
    assert!(report["additional_context"]
        .as_str()
        .unwrap()
        .contains("/context/fault"));
    assert!(report["additional_context"]
        .as_str()
        .unwrap()
        .contains("subagents"));
    assert!(
        report["additional_context"].as_str().unwrap().len() < 900,
        "hook context should stay compact"
    );

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_nano_returns_under_10_token_carrier_and_expands() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let block = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args([
            "context",
            "nano",
            "login route session audit",
            "--budget-tokens",
            "320",
            "--block",
        ])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run context nano block");
    assert!(
        block.status.success(),
        "context nano --block failed: stderr={}",
        String::from_utf8_lossy(&block.stderr)
    );
    let carrier = String::from_utf8(block.stdout)
        .expect("utf8 carrier")
        .trim()
        .to_string();
    assert!(carrier.starts_with("qfx:"));
    assert!(
        (carrier.len() as u64).div_ceil(4) <= 10,
        "carrier should stay under 10 estimated tokens: {carrier}"
    );

    let nano = qorx(
        &[
            "context",
            "nano",
            "login route session audit",
            "--budget-tokens",
            "320",
        ],
        &qorx_home,
    );
    assert_eq!(nano["schema"], "qorx.context-nano.v1");
    assert_eq!(nano["version"], "1.0.0");
    assert_eq!(nano["carrier"], carrier);
    assert!(nano["visible_tokens"].as_u64().unwrap() <= 10);
    assert!(nano["context_reduction_x"].as_f64().unwrap() > 10.0);
    assert!(nano["boundary"].as_str().unwrap().contains("pointer"));

    let expanded = qorx(&["context", "expand", &carrier], &qorx_home);
    assert_eq!(expanded["schema"], "qorx.context-expand.v1");
    assert_eq!(expanded["version"], "1.0.0");
    assert_eq!(expanded["authorized"], true);
    assert_eq!(expanded["carrier"], carrier);
    assert!(expanded["handle"]
        .as_str()
        .unwrap()
        .starts_with("qorx://s/"));
    assert!(expanded["contract"]["fault_endpoint"]
        .as_str()
        .unwrap()
        .contains("/context/fault"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_fault_accepts_nano_carrier_and_returns_proof_pages() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);
    let nano = qorx(
        &[
            "context",
            "nano",
            "production gate routed provider evidence",
            "--budget-tokens",
            "180",
        ],
        &qorx_home,
    );
    let carrier = nano["carrier"].as_str().unwrap();

    let report = qorx(
        &[
            "context",
            "fault",
            "production gate routed provider evidence",
            "--handle",
            carrier,
            "--budget-tokens",
            "180",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-fault.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["authorized"], true);
    assert_eq!(report["carrier"], carrier);
    assert!(report["handle"].as_str().unwrap().starts_with("qorx://s/"));
    assert_eq!(report["proof_pages"][0]["path"], "src/money.rs");

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_fault_rejects_stale_nano_carrier() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let report = qorx(
        &[
            "context",
            "fault",
            "production gate routed provider evidence",
            "--handle",
            "qfx:deadbeef",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-fault.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["status"], "unauthorized");
    assert_eq!(report["authorized"], false);
    assert_eq!(report["carrier"], "qfx:deadbeef");
    assert!(report["proof_pages"].as_array().unwrap().is_empty());
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("active qfx nano carrier"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_nano_addresses_1m_local_tokens_with_under_10_visible_tokens() {
    let qorx_home = unique_temp_dir();
    fs::create_dir_all(&qorx_home).expect("create qorx home");
    fs::write(
        qorx_home.join("repo_index.json"),
        r#"{
  "root": "C:/repo",
  "updated_at": "2026-05-01T00:00:00Z",
  "quarks": [
    {
      "id": "qva_million",
      "path": "docs/mega-context.md",
      "start_line": 1,
      "end_line": 4,
      "hash": "millionhash",
      "token_estimate": 1000000,
      "symbols": ["MegaContext"],
      "signal_mask": 0,
      "vector": [51, 52, 53],
      "text": "mega context proof page says Qorx keeps one million local tokens behind a nano carrier and resolves evidence through context faults"
    }
  ]
}"#,
    )
    .expect("write 1m index");

    let report = qorx(
        &[
            "context",
            "nano",
            "mega context proof page",
            "--budget-tokens",
            "320",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-nano.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["indexed_tokens"], 1_000_000);
    assert!(report["visible_tokens"].as_u64().unwrap() <= 10);
    assert!(report["context_reduction_x"].as_f64().unwrap() >= 100_000.0);
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("does not contain the local context"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_quetta_returns_one_token_alias_and_counterfactual_ledger() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let block = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(["context", "quetta", "--block"])
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run context quetta block");
    assert!(
        block.status.success(),
        "context quetta --block failed: stderr={}",
        String::from_utf8_lossy(&block.stderr)
    );
    let alias = String::from_utf8(block.stdout)
        .expect("utf8 alias")
        .trim()
        .to_string();
    assert_eq!(alias, "Q");
    assert_eq!(alias.chars().count(), 1);

    let report = qorx(
        &[
            "context",
            "quetta",
            "production gate routed provider evidence",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-quetta.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["carrier"], "Q");
    assert_eq!(report["visible_tokens"], 1);
    assert_eq!(
        report["manifest"]["logical_context_bytes"],
        "1000000000000000000000000000000"
    );
    assert_eq!(
        report["value_ledger"]["counterfactual_value_usd"],
        "1000000000000000000000000000000000"
    );
    assert_eq!(report["value_ledger"]["visible_alias_cost_usd"], "0.001");
    assert_eq!(
        report["value_ledger"]["effective_leverage_x"],
        "1000000000000000000000000000000000000"
    );
    assert_eq!(report["value_ledger"]["billing_claim"], false);
    assert_eq!(report["manifest"]["physical_manifest_present"], false);
    assert_eq!(report["manifest"]["lossless_resolver"], false);
    assert!(report["manifest"]["boundary"]
        .as_str()
        .unwrap()
        .contains("counterfactual"));
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("opens the vault"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn context_quetta_scales_synthetic_big_token_indexes_without_overflow() {
    for token_estimate in [50_000_000_u64, 1_000_000_000_u64] {
        let qorx_home = unique_temp_dir();
        seed_synthetic_scale_index(&qorx_home, token_estimate);

        let report = qorx(
            &["context", "quetta", "synthetic scale sentinel"],
            &qorx_home,
        );
        assert_eq!(report["schema"], "qorx.context-quetta.v1");
        assert_eq!(report["version"], "1.0.0");
        assert_eq!(report["carrier"], "Q");
        assert_eq!(report["visible_tokens"], 1);
        assert_eq!(report["local_indexed_tokens"], token_estimate);
        assert_eq!(report["manifest"]["indexed_tokens"], token_estimate);
        assert_eq!(report["manifest"]["quark_count"], 1);
        assert_eq!(report["manifest"]["physical_manifest_present"], false);
        assert_eq!(report["manifest"]["lossless_resolver"], false);

        let fault = qorx(
            &[
                "context",
                "fault",
                "synthetic scale sentinel",
                "--handle",
                "Q",
                "--budget-tokens",
                "180",
            ],
            &qorx_home,
        );
        assert_eq!(fault["schema"], "qorx.context-fault.v1");
        assert_eq!(fault["authorized"], true);
        assert_eq!(fault["carrier"], "Q");
        assert_eq!(fault["indexed_tokens"], token_estimate);
        assert_eq!(fault["proof_pages"][0]["path"], "docs/synthetic-scale.md");

        let _ = fs::remove_dir_all(&qorx_home);
    }
}

#[test]
fn context_expand_and_fault_accept_quetta_alias() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    let expanded = qorx(&["context", "expand", "Q"], &qorx_home);
    assert_eq!(expanded["schema"], "qorx.context-expand.v1");
    assert_eq!(expanded["version"], "1.0.0");
    assert_eq!(expanded["authorized"], true);
    assert_eq!(expanded["carrier"], "Q");
    assert_eq!(expanded["authorization"], "active-quetta-alias");
    assert_eq!(expanded["manifest"]["alias"], "Q");
    assert_eq!(
        expanded["manifest"]["logical_context_bytes"],
        "1000000000000000000000000000000"
    );

    let report = qorx(
        &[
            "context",
            "fault",
            "production gate routed provider evidence",
            "--handle",
            "Q",
            "--budget-tokens",
            "180",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.context-fault.v1");
    assert_eq!(report["version"], "1.0.0");
    assert_eq!(report["authorized"], true);
    assert_eq!(report["carrier"], "Q");
    assert_eq!(report["authorization"], "active-quetta-alias");
    assert_eq!(report["proof_pages"][0]["path"], "src/money.rs");

    let _ = fs::remove_dir_all(&qorx_home);
}
