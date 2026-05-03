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
        "qorx-lattice-{}-{suffix}-{sequence}",
        std::process::id()
    ))
}

fn seed_index(dir: &Path) {
    fs::create_dir_all(dir).expect("create qorx home");
    fs::write(
        dir.join("repo_index.json"),
        r#"{
  "root": "C:/repo",
  "updated_at": "2026-04-29T00:00:00Z",
  "quarks": [
    {
      "id": "qva_auth_route",
      "path": "src/routes/auth.ts",
      "start_line": 1,
      "end_line": 8,
      "hash": "authhash",
      "token_estimate": 44,
      "symbols": ["loginRoute", "issueSession"],
      "signal_mask": 66,
      "vector": [21, 22, 23],
      "text": "export function loginRoute(req) {\n  const session = issueSession(req.user);\n  logAudit(session.id);\n  return session;\n}"
    },
    {
      "id": "qva_session_service",
      "path": "src/services/session.ts",
      "start_line": 1,
      "end_line": 5,
      "hash": "sessionhash",
      "token_estimate": 32,
      "symbols": ["issueSession"],
      "signal_mask": 64,
      "vector": [31, 32, 33],
      "text": "export function issueSession(user) {\n  return { id: user.id, expires: Date.now() + 3600 };\n}"
    },
    {
      "id": "qva_billing",
      "path": "src/billing.ts",
      "start_line": 1,
      "end_line": 3,
      "hash": "billinghash",
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

fn qorx_text(args: &[&str], qorx_home: &Path) -> String {
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn memory_evolve_builds_deterministic_lattice_with_provenance_and_kv_hints() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    qorx(
        &[
            "memory",
            "create",
            "decision",
            "auth refactor must preserve loginRoute session audit behavior",
        ],
        &qorx_home,
    );

    let report = qorx(
        &[
            "memory",
            "evolve",
            "--task",
            "refactor auth login session audit",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.memory.evolve.v1");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert_eq!(report["lattice"]["schema"], "qorx.lattice.v1");
    assert!(report["lattice"]["handle"]
        .as_str()
        .unwrap()
        .starts_with("qorx://l/"));
    assert!(report["lattice"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["layer"] == 2 && node["kind"] == "memento"));
    assert!(report["lattice"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["layer"] == 3 && node["kind"] == "strategy"));
    assert!(
        report["lattice"]["b2c"]["local_idx_tokens"]
            .as_u64()
            .unwrap()
            > report["lattice"]["b2c"]["visible_tokens"].as_u64().unwrap()
    );
    assert_eq!(
        report["lattice"]["kv_hints"]["realized_kv_compression"],
        false
    );
    assert!(report["lattice"]["prompt_block"]
        .as_str()
        .unwrap()
        .starts_with("QORX_LATTICE qorx://l/"));
    assert!(qorx_home.join("qorx-lattice.pb").exists());

    let _ = fs::remove_dir_all(qorx_home);
}

#[test]
fn lattice_formal_attest_verifies_hashes_provenance_and_b2c_math() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    qorx(
        &[
            "memory",
            "evolve",
            "--task",
            "refactor auth login session audit",
        ],
        &qorx_home,
    );
    let report = qorx(&["lattice", "attest", "--formal"], &qorx_home);

    assert_eq!(report["schema"], "qorx.lattice.attestation.v1");
    assert_eq!(report["formal"], true);
    assert_eq!(report["verified"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["certificate_sha256"].as_str().unwrap().len() >= 64);
    for check in report["checks"].as_array().unwrap() {
        assert_eq!(check["passed"], true, "failed check: {check}");
    }

    let _ = fs::remove_dir_all(qorx_home);
}

#[test]
fn top_level_attest_and_kv_emit_expose_v1_contextos_surface() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    qorx(
        &[
            "memory",
            "evolve",
            "--task",
            "refactor auth login session audit",
        ],
        &qorx_home,
    );

    let attest = qorx(&["attest", "--formal", "--level", "3"], &qorx_home);
    assert_eq!(attest["schema"], "qorx.formal-attestation.v1");
    assert_eq!(attest["formal"], true);
    assert_eq!(attest["level"], 3);
    assert_eq!(attest["verified"], true);
    assert_eq!(attest["provider_calls"], 0);
    assert!(
        attest["lattice"]["certificate_sha256"]
            .as_str()
            .unwrap()
            .len()
            >= 64
    );

    let safetensors = qorx_home.join("qorx-kv-hints.safetensors");
    let safetensors_text = safetensors.display().to_string();
    let kv = qorx(
        &["kv", "emit", "--model", "vllm", "--out", &safetensors_text],
        &qorx_home,
    );
    assert_eq!(kv["schema"], "qorx.kv.emit.v1");
    assert_eq!(kv["model"], "vllm");
    assert_eq!(kv["local_only"], true);
    assert_eq!(kv["provider_calls"], 0);
    assert_eq!(kv["realized_kv_compression"], false);
    assert!(!kv["hints"].as_array().unwrap().is_empty());
    assert_eq!(kv["safetensors"]["written"], true);
    assert!(safetensors.exists());
    let bytes = fs::read(&safetensors).expect("read safetensors");
    assert!(bytes.len() > 16);

    let _ = fs::remove_dir_all(qorx_home);
}

#[test]
fn memory_gc_lattice_deduplicates_memory_without_provider_calls() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    qorx(
        &[
            "memory",
            "create",
            "decision",
            "duplicate lattice memory should collapse",
        ],
        &qorx_home,
    );
    qorx(
        &[
            "memory",
            "create",
            "decision",
            "duplicate lattice memory should collapse",
        ],
        &qorx_home,
    );

    let report = qorx(
        &["memory", "gc", "--strategy", "lattice", "--max-items", "8"],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.memory.v1");
    assert_eq!(report["action"], "gc");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert_eq!(report["items_kept"], 1);
    assert_eq!(report["pruned_count"], 1);
    assert!(report["boundary"]
        .as_str()
        .unwrap()
        .contains("raw quark evidence"));

    let _ = fs::remove_dir_all(qorx_home);
}

#[test]
fn lattice_evolve_rules_updates_deterministic_rules_from_local_metrics() {
    let qorx_home = unique_temp_dir();
    seed_index(&qorx_home);

    qorx(
        &[
            "memory",
            "evolve",
            "--task",
            "refactor auth login session audit",
        ],
        &qorx_home,
    );
    let report = qorx(
        &[
            "lattice",
            "evolve-rules",
            "--task",
            "refactor auth login session audit",
        ],
        &qorx_home,
    );

    assert_eq!(report["schema"], "qorx.lattice.rules.v1");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert!(report["generation"].as_u64().unwrap() >= 1);
    assert!(report["rules"]
        .as_array()
        .unwrap()
        .iter()
        .any(|rule| rule["name"] == "promote_multi_source_mementos"));
    assert!(report["metrics"]["coherence_score"].as_f64().unwrap() > 0.0);
    assert!(report["rules_protobuf"]
        .as_str()
        .unwrap()
        .ends_with("qorx-lattice-rules.pb"));
    assert!(qorx_home.join("qorx-lattice-rules.pb").exists());

    let status = qorx(&["lattice", "rules"], &qorx_home);
    assert_eq!(status["schema"], "qorx.lattice.rules.v1");
    assert_eq!(status["generation"], report["generation"]);

    let _ = fs::remove_dir_all(qorx_home);
}

#[test]
fn share_export_import_federates_lattice_capsules_through_local_file_only() {
    let source_home = unique_temp_dir();
    let peer_home = unique_temp_dir();
    let capsule_root = unique_temp_dir();
    seed_index(&source_home);
    seed_index(&peer_home);
    fs::create_dir_all(&capsule_root).expect("create capsule root");
    fs::write(
        capsule_root.join("notes.md"),
        "federated auth capsule keeps login session audit provenance local",
    )
    .expect("write capsule note");

    let capsule_root_text = capsule_root.display().to_string();
    qorx_text(
        &[
            "capsule",
            "create",
            &capsule_root_text,
            "--include-memory",
            "--block",
        ],
        &source_home,
    );
    qorx(
        &[
            "memory",
            "evolve",
            "--task",
            "federated auth login session audit",
        ],
        &source_home,
    );
    qorx(
        &[
            "lattice",
            "evolve-rules",
            "--task",
            "federated auth login session audit",
        ],
        &source_home,
    );

    let bundle = source_home.join("qorx-share.pb");
    let bundle_text = bundle.display().to_string();
    let capsule_status = qorx(&["capsule", "session"], &source_home);
    let capsule_handle = capsule_status["handle"].as_str().unwrap();
    let export_report = qorx(
        &[
            "share",
            "capsule",
            "--capsule",
            capsule_handle,
            "--to",
            &bundle_text,
        ],
        &source_home,
    );
    assert_eq!(export_report["schema"], "qorx.share.export.v1");
    assert_eq!(export_report["local_only"], true);
    assert_eq!(export_report["provider_calls"], 0);
    assert_eq!(export_report["transport"], "local_file");
    assert_eq!(export_report["capsules"], 1);
    assert!(bundle.exists());

    let import_report = qorx(&["share", "import", &bundle_text], &peer_home);
    assert_eq!(import_report["schema"], "qorx.share.import.v1");
    assert_eq!(import_report["local_only"], true);
    assert_eq!(import_report["provider_calls"], 0);
    assert_eq!(import_report["transport"], "local_file");
    assert!(import_report["imported_nodes"].as_u64().unwrap() > 0);
    assert_eq!(import_report["federated_capsules"].as_u64().unwrap(), 1);
    assert!(import_report["merged_lattice"]["handle"]
        .as_str()
        .unwrap()
        .starts_with("qorx://l/"));

    let prompt = qorx_text(&["share", "session", "--block"], &peer_home);
    assert!(prompt.starts_with("QORX_FEDERATION qorx://f/"));
    assert!(prompt.contains("transport=local_file"));

    let _ = fs::remove_dir_all(source_home);
    let _ = fs::remove_dir_all(peer_home);
    let _ = fs::remove_dir_all(capsule_root);
}
