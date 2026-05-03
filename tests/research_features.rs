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
        "qorx-research-features-{}-{suffix}-{sequence}",
        std::process::id()
    ))
}

fn seed_research_index(dir: &Path) {
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
    },
    {
      "id": "qva_unrelated",
      "path": "src/billing.ts",
      "start_line": 1,
      "end_line": 3,
      "hash": "mno",
      "token_estimate": 20,
      "symbols": ["billCustomer"],
      "signal_mask": 0,
      "vector": [51, 52, 53],
      "text": "export function billCustomer(customer) { return customer.plan; }"
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
fn squeeze_returns_query_aware_extracts_under_budget() {
    let qorx_home = unique_temp_dir();
    seed_research_index(&qorx_home);

    let report = qorx(
        &[
            "squeeze",
            "production gate routed provider evidence",
            "--budget-tokens",
            "180",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.squeeze.v1");
    assert_eq!(report["mode"], "extractive_query_squeeze");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["used_tokens"].as_u64().unwrap() <= 180);
    assert!(
        report["squeezed_tokens"].as_u64().unwrap() < report["source_tokens"].as_u64().unwrap()
    );
    assert!(report["text"]
        .as_str()
        .unwrap()
        .contains("production gate requires routed provider savings evidence"));
    assert_eq!(report["quarks_used"], 1);
    assert!(!report["text"]
        .as_str()
        .unwrap()
        .contains("filler unrelated screen copy"));
    assert_eq!(report["evidence"][0]["id"], "qva_money");

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn judge_marks_supported_and_unsupported_claims() {
    let qorx_home = unique_temp_dir();
    seed_research_index(&qorx_home);

    let report = qorx(
        &[
            "judge",
            "production gate requires routed provider savings evidence. warp drive cooking schedule is approved.",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.judge.v1");
    assert_eq!(report["unsupported_claims"], 1);
    assert_eq!(report["claims"][0]["verdict"], "supported");
    assert_eq!(report["claims"][1]["verdict"], "unsupported");
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("indexed local evidence"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn cache_plan_splits_stable_prefix_from_dynamic_tail() {
    let qorx_home = unique_temp_dir();
    fs::create_dir_all(&qorx_home).expect("create home");
    let prompt = "system: use qorx session pointer\npolicy: stable cache prefix\n--- QORX_DYNAMIC ---\nuser asks live question";

    let report = qorx(&["cache-plan", prompt], &qorx_home);

    assert_eq!(report["schema"], "qorx.cache-plan.v1");
    assert_eq!(report["marker"], "--- QORX_DYNAMIC ---");
    assert_eq!(report["can_cache_prefix"], true);
    assert!(report["stable_prefix_tokens"].as_u64().unwrap() > 0);
    assert!(report["dynamic_tail_tokens"].as_u64().unwrap() > 0);
    assert!(report["recommendations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("stable prefix first")));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn b2c_plan_runs_quant_allocator_over_indexed_quarks() {
    let qorx_home = unique_temp_dir();
    seed_research_index(&qorx_home);

    let report = qorx(
        &[
            "b2c-plan",
            "login route session audit",
            "--budget-tokens",
            "220",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.b2c-plan.v1");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["used_tokens"].as_u64().unwrap() <= 220);
    assert!(report["selected_quarks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["path"] == "src/routes/auth.ts"));
    assert!(report["parallel_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane["name"] == "portfolio"));
    assert_eq!(report["math"]["budget_model"], "bounded_knapsack");
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("deterministic local math"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn pack_carries_b2c_allocator_proof_in_the_hot_context() {
    let qorx_home = unique_temp_dir();
    seed_research_index(&qorx_home);

    let report = qorx(
        &[
            "pack",
            "login route session audit",
            "--budget-tokens",
            "220",
        ],
        &qorx_home,
    );

    assert_eq!(report["query"], "login route session audit");
    assert!(report["text"]
        .as_str()
        .unwrap()
        .contains("# Qorx B2C packed context"));
    assert!(report["text"]
        .as_str()
        .unwrap()
        .contains("b2c_parallel_lanes=retrieval,portfolio,risk,cache,carrier"));
    assert!(report["text"]
        .as_str()
        .unwrap()
        .contains("b2c_math=bounded_knapsack"));
    assert!(report["quarks_used"].as_u64().unwrap() >= 1);

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn map_reports_changed_paths_symbols_and_related_edges() {
    let qorx_home = unique_temp_dir();
    seed_research_index(&qorx_home);
    let diff_file = qorx_home.join("auth.diff");
    fs::write(
        &diff_file,
        "diff --git a/src/routes/auth.ts b/src/routes/auth.ts\n+++ b/src/routes/auth.ts\n@@\n+  logAudit(session.id);\n",
    )
    .expect("write diff");

    let diff_file_text = diff_file.display().to_string();
    let report = qorx(
        &[
            "map",
            "login route session audit",
            "--diff-file",
            &diff_file_text,
            "--budget-tokens",
            "320",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.map.v1");
    assert_eq!(report["changed_paths"][0], "src/routes/auth.ts");
    assert!(report["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .any(|symbol| symbol["name"] == "issueSession"));
    assert!(report["graph_edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| {
            edge["from_path"] == "src/routes/auth.ts"
                && edge["to_path"] == "src/services/session.ts"
        }));
    assert!(!report["text"].as_str().unwrap().contains("billCustomer"));

    let _ = fs::remove_dir_all(&qorx_home);
}

#[test]
fn memory_crud_summarize_and_prune_are_local() {
    let qorx_home = unique_temp_dir();
    fs::create_dir_all(&qorx_home).expect("create home");

    let created = qorx(
        &[
            "memory",
            "create",
            "decision",
            "provider traffic routes through Qorx before money claims",
        ],
        &qorx_home,
    );
    assert_eq!(created["schema"], "qorx.memory.v1");
    assert_eq!(created["action"], "create");
    assert_eq!(created["local_only"], true);
    let id = created["item"]["id"].as_str().unwrap().to_string();

    let read = qorx(&["memory", "read", "provider traffic"], &qorx_home);
    assert_eq!(read["items"].as_array().unwrap().len(), 1);
    assert_eq!(read["items"][0]["id"], id);

    let updated = qorx(
        &[
            "memory",
            "update",
            &id,
            "provider traffic routes through Qorx and records Baseline-to-Compact proof",
        ],
        &qorx_home,
    );
    assert_eq!(updated["action"], "update");
    assert!(updated["item"]["text"]
        .as_str()
        .unwrap()
        .contains("Baseline-to-Compact proof"));

    let summary = qorx(&["memory", "summarize"], &qorx_home);
    assert_eq!(summary["action"], "summarize");
    assert!(summary["summary"]
        .as_str()
        .unwrap()
        .contains("Baseline-to-Compact proof"));

    let pruned = qorx(&["memory", "prune", "--max-items", "1"], &qorx_home);
    assert_eq!(pruned["action"], "prune");
    assert_eq!(pruned["items_kept"], 1);

    let deleted = qorx(&["memory", "delete", &id], &qorx_home);
    assert_eq!(deleted["action"], "delete");
    assert_eq!(deleted["deleted"], true);

    let empty = qorx(&["memory", "read", "provider traffic"], &qorx_home);
    assert!(empty["items"].as_array().unwrap().is_empty());

    let _ = fs::remove_dir_all(&qorx_home);
}
