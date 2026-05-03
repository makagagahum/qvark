use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
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
        "qorx-language-{}-{suffix}-{sequence}",
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
      "id": "qva_money",
      "path": "src/money.rs",
      "start_line": 1,
      "end_line": 3,
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
      "end_line": 6,
      "hash": "def",
      "token_estimate": 24,
      "symbols": ["deterministic_subatomic_agent"],
      "signal_mask": 64,
      "vector": [21, 22, 23],
      "text": "the qorx agent executes local plans with strict-answer and pack steps"
    }
  ]
}"#,
    )
    .expect("write index");
    fs::write(dir.join("stats.json"), r#"{"requests":0}"#).expect("write stats");
}

fn qorx_raw(args: &[&str], qorx_home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args(args)
        .env("QORX_HOME", qorx_home)
        .output()
        .unwrap_or_else(|err| panic!("run qorx {args:?}: {err}"))
}

fn qorx(args: &[&str], qorx_home: &Path) -> serde_json::Value {
    let output = qorx_raw(args, qorx_home);
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
fn qorx_check_reports_ast_and_qir_for_language_program() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("compiler.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
strict answer from evidence limit 1
assert supported(answer)
emit answer
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["schema"], "qorx.check.v1");
    assert_eq!(report["language"], "qorx");
    assert_eq!(report["valid"], true);
    assert_eq!(report["diagnostics"].as_array().unwrap().len(), 0);
    assert_eq!(report["ast"][0]["kind"], "version");
    assert!(report["ast"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "assert-supported" && node["target"] == "answer"));
    assert!(report["qir"]
        .as_array()
        .unwrap()
        .iter()
        .any(|instruction| instruction["op"] == "CALL_PACK"
            && instruction["target"] == "evidence"
            && instruction["source"] == "question"));
    assert!(report["qir"]
        .as_array()
        .unwrap()
        .iter()
        .any(|instruction| instruction["op"] == "ASSERT_SUPPORTED"
            && instruction["target"] == "answer"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_check_reports_cache_policy_in_ast_and_qir() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("cache-policy.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
cache evidence key question ttl 3600
strict answer from evidence limit 1
emit answer
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["valid"], true);
    assert!(report["ast"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "cache-policy"
            && node["target"] == "evidence"
            && node["source"] == "question"));
    assert!(report["qir"]
        .as_array()
        .unwrap()
        .iter()
        .any(|instruction| instruction["op"] == "CACHE_BIND"
            && instruction["target"] == "evidence"
            && instruction["source"] == "question"
            && instruction["args"]["ttl_seconds"] == "3600"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_check_reports_supported_branch_control_flow() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("branch.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
strict answer from question limit 1
if supported(answer) then emit answer else emit fallback
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["valid"], true);
    assert!(report["ast"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "branch"
            && node["op"] == "supported"
            && node["source"] == "answer"
            && node["target"] == "answer"
            && node["else_target"] == "fallback"));
    assert!(report["qir"]
        .as_array()
        .unwrap()
        .iter()
        .any(|instruction| instruction["op"] == "IF_SUPPORTED"
            && instruction["source"] == "answer"
            && instruction["target"] == "answer"
            && instruction["args"]["else_emit"] == "fallback"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_check_reports_stdlib_imports_in_ast_and_qir() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("imports.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
use std.evidence
use std.branch as br
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
strict answer from question limit 1
if supported(answer) then emit answer else emit fallback
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["valid"], true);
    assert_eq!(report["program"]["imports"][0]["module"], "std.evidence");
    assert_eq!(report["program"]["imports"][1]["module"], "std.branch");
    assert_eq!(report["program"]["imports"][1]["alias"], "br");
    assert!(report["ast"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "import" && node["name"] == "std.evidence"));
    assert!(report["qir"]
        .as_array()
        .unwrap()
        .iter()
        .any(|instruction| instruction["op"] == "IMPORT_MODULE"
            && instruction["source"] == "std.branch"
            && instruction["target"] == "br"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_check_rejects_unknown_stdlib_imports() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("bad-import.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
use std.telepathy
let question = "production gate routed provider evidence"
strict answer from question limit 1
emit answer
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["valid"], false);
    assert!(report["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("unknown Qorx std module `std.telepathy`"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_compile_preserves_import_opcodes_in_bytecode() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("import-bytecode.qorx");
    let bytecode_file = root.join("import-bytecode.qorxb");
    fs::write(
        &qorx_file,
        r#"QORX 1
use std.evidence
use std.branch as br
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
strict answer from question limit 1
if supported(answer) then emit answer else emit fallback
"#,
    )
    .expect("write qorx file");

    let compile = qorx(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );

    let opcodes = compile["bytecode"]["opcodes"].as_array().expect("opcodes");
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "IMPORT" && op["value"] == "std.evidence"));
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "IMPORT_AS" && op["value"] == "std.branch as br"));

    let run = qorx(&["qorx", bytecode_file.to_str().unwrap()], &qorx_home);

    assert_eq!(run["source_kind"], "qorxb");
    assert_eq!(run["execution"]["imports"][0]["module"], "std.evidence");
    assert_eq!(run["execution"]["imports"][1]["alias"], "br");
    assert_eq!(run["execution"]["branches"][0]["taken"], "then");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_compile_emits_forth_like_qstk_in_protobuf_bytecode() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("qstk-bytecode.qorx");
    let bytecode_file = root.join("qstk-bytecode.qorxb");
    fs::write(
        &qorx_file,
        r#"QORX 1
use std.evidence
use std.branch as br
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
strict answer from question limit 1
if supported(answer) then emit answer else emit fallback
"#,
    )
    .expect("write qorx file");

    let compile = qorx(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );

    assert_eq!(compile["bytecode"]["qstk_hash"].as_str().unwrap().len(), 64);
    let qstk = compile["bytecode"]["qstk"].as_array().expect("qstk");
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "use" && word["arg"] == "std.evidence"));
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "usas" && word["arg"] == "std.branch br"));
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "lit" && word["arg"].as_str().unwrap().len() == 64));
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "call" && word["arg"] == "STRICT_ANSWER answer<-question"));
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "qif" && word["arg"] == "supported(answer)"));
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "then" && word["arg"] == "answer"));
    assert!(qstk
        .iter()
        .any(|word| word["word"] == "qels" && word["arg"] == "fallback"));

    let inspect = qorx(
        &["qorx-inspect", bytecode_file.to_str().unwrap()],
        &qorx_home,
    );

    assert_eq!(inspect["source_kind"], "qorxb");
    assert_eq!(inspect["qstk_hash"], compile["bytecode"]["qstk_hash"]);
    assert_eq!(inspect["qstk"].as_array().unwrap().len(), qstk.len());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_program_branch_emits_fallback_when_answer_is_unsupported() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("branch-fallback.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "galactic banana escrow treaty"
let fallback = "qv0d: local evidence does not support this answer"
strict answer from question limit 1
if supported(answer) then emit answer else emit fallback
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["execution"]["emitted"], "fallback");
    assert_eq!(report["execution"]["branches"][0]["predicate"], "supported");
    assert_eq!(report["execution"]["branches"][0]["condition"], "answer");
    assert_eq!(report["execution"]["branches"][0]["passed"], false);
    assert_eq!(report["execution"]["branches"][0]["taken"], "else");
    assert_eq!(
        report["execution"]["output"]["value"],
        "qv0d: local evidence does not support this answer"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_compile_preserves_branch_opcodes_in_bytecode() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("branch-bytecode.qorx");
    let bytecode_file = root.join("branch-bytecode.qorxb");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
let fallback = "qv0d: local evidence does not support this answer"
strict answer from question limit 1
if supported(answer) then emit answer else emit fallback
"#,
    )
    .expect("write qorx file");

    let compile = qorx(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );

    let opcodes = compile["bytecode"]["opcodes"].as_array().expect("opcodes");
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "IF_SUPPORTED" && op["value"] == "answer"));
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "THEN_EMIT" && op["value"] == "answer"));
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "ELSE_EMIT" && op["value"] == "fallback"));

    let run = qorx(&["qorx", bytecode_file.to_str().unwrap()], &qorx_home);

    assert_eq!(run["source_kind"], "qorxb");
    assert_eq!(run["execution"]["emitted"], "answer");
    assert_eq!(run["execution"]["branches"][0]["taken"], "then");
    assert_eq!(run["execution"]["output"]["coverage"], "supported");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_cache_policy_rejects_undefined_cache_keys() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("bad-cache.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
cache evidence key missing ttl 3600
emit evidence
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["valid"], false);
    assert!(report["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("undefined Qorx symbol `missing` used as cache key"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_check_rejects_undefined_symbols_before_compile() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("bad.qorx");
    let bytecode_file = root.join("bad.qorxb");
    fs::write(
        &qorx_file,
        r#"QORX 1
strict answer from missing limit 1
emit answer
"#,
    )
    .expect("write qorx file");

    let check = qorx(&["qorx-check", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(check["valid"], false);
    assert_eq!(check["diagnostics"][0]["severity"], "error");
    assert!(check["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("undefined Qorx symbol `missing`"));

    let compile = qorx_raw(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );

    assert!(!compile.status.success());
    assert!(String::from_utf8_lossy(&compile.stderr).contains("undefined Qorx symbol `missing`"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_file_runs_agent_mode_as_qorx_language() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("god-idea.qorx");
    fs::write(
        &qorx_file,
        "QORX 1\n@mode agent\n@handle qorx://s/test\n@goal prove production gate provider savings\n@budget 600\n",
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["schema"], "qorx.run.v1");
    assert_eq!(report["language"], "qorx");
    assert_eq!(report["extension"], ".qorx");
    assert_eq!(report["local_only"], true);
    assert_eq!(report["provider_calls"], 0);
    assert_eq!(report["program"]["mode"], "agent");
    assert_eq!(report["program"]["handle"], "qorx://s/test");
    assert!(report["visible_tokens"].as_u64().unwrap() < 64);
    assert_eq!(report["execution"]["schema"], "qorx.agent.v1");
    assert_eq!(report["execution"]["provider_calls"], 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_file_can_ask_strict_evidence_question() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("strict.qorx");
    fs::write(
        &qorx_file,
        "mode: strict-answer\nask: production gate routed provider evidence\nlimit: 1\n",
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["program"]["mode"], "strict-answer");
    assert_eq!(report["execution"]["schema"], "qorx.strict-answer.v1");
    assert_eq!(report["execution"]["coverage"], "supported");
    assert_eq!(report["execution"]["evidence"][0]["id"], "qva_money");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_compile_emits_protobuf_bytecode_and_runs_it() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("god-idea.qorx");
    let bytecode_file = root.join("god-idea.qorxb");
    fs::write(
        &qorx_file,
        "QORX 1\n@mode agent\n@handle qorx://s/test\n@goal prove production gate provider savings\n@budget 600\n",
    )
    .expect("write qorx file");

    let compile = qorx(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );

    assert_eq!(compile["schema"], "qorx.compile.v1");
    assert_eq!(compile["language"], "qorx");
    assert_eq!(compile["bytecode"]["schema"], "qorx.bytecode.v1");
    assert_eq!(compile["bytecode"]["instruction_count"], 7);
    assert_eq!(
        compile["bytecode"]["opcodes_hash"].as_str().unwrap().len(),
        64
    );
    assert_eq!(
        compile["bytecode"]["program_hash"].as_str().unwrap().len(),
        64
    );
    assert_eq!(compile["output"], bytecode_file.display().to_string());
    assert!(compile["bytecode_bytes"].as_u64().unwrap() > 0);
    assert!(bytecode_file.exists());

    let run = qorx(&["qorx", bytecode_file.to_str().unwrap()], &qorx_home);

    assert_eq!(run["schema"], "qorx.run.v1");
    assert_eq!(run["source_kind"], "qorxb");
    assert_eq!(run["program"]["mode"], "agent");
    assert_eq!(run["execution"]["schema"], "qorx.agent.v1");
    assert_eq!(run["provider_calls"], 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_program_language_runs_named_resolver_steps() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("ai-context.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
strict answer from evidence limit 1
emit answer
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["program"]["mode"], "program");
    assert_eq!(report["program"]["bindings"][0]["name"], "question");
    assert_eq!(report["program"]["steps"][0]["name"], "evidence");
    assert_eq!(report["program"]["steps"][0]["op"], "pack");
    assert_eq!(report["program"]["steps"][1]["name"], "answer");
    assert_eq!(report["program"]["steps"][1]["op"], "strict-answer");
    assert_eq!(report["execution"]["schema"], "qorx.program-execution.v1");
    assert_eq!(report["execution"]["emitted"], "answer");
    assert_eq!(report["execution"]["steps"][0]["op"], "pack");
    assert_eq!(report["execution"]["steps"][1]["op"], "strict-answer");
    assert_eq!(report["execution"]["output"]["coverage"], "supported");
    assert_eq!(
        report["execution"]["output"]["evidence"][0]["id"],
        "qva_money"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_program_assert_supported_gates_execution() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("assert-supported.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
strict answer from question limit 1
assert supported(answer)
emit answer
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(
        report["execution"]["assertions"][0]["predicate"],
        "supported"
    );
    assert_eq!(report["execution"]["assertions"][0]["target"], "answer");
    assert_eq!(report["execution"]["assertions"][0]["passed"], true);
    assert_eq!(report["execution"]["output"]["coverage"], "supported");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_program_emits_cache_trace_for_cached_steps() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("cache-trace.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
cache evidence key question ttl 3600
strict answer from evidence limit 1
emit answer
"#,
    )
    .expect("write qorx file");

    let report = qorx(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["execution"]["cache"][0]["target"], "evidence");
    assert_eq!(report["execution"]["cache"][0]["key_source"], "question");
    assert_eq!(report["execution"]["cache"][0]["ttl_seconds"], 3600);
    assert!(report["execution"]["cache"][0]["key"]
        .as_str()
        .unwrap()
        .starts_with("qrc_"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_program_assert_supported_rejects_unsupported_answers() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("assert-unsupported.qorx");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "galactic banana escrow treaty"
strict answer from question limit 1
assert supported(answer)
emit answer
"#,
    )
    .expect("write qorx file");

    let output = qorx_raw(&["qorx", qorx_file.to_str().unwrap()], &qorx_home);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("assert supported(answer) failed"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_compile_preserves_named_resolver_opcodes_in_bytecode() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("ai-context.qorx");
    let bytecode_file = root.join("ai-context.qorxb");
    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
strict answer from evidence limit 1
emit answer
"#,
    )
    .expect("write qorx file");

    let compile = qorx(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );

    let opcodes = compile["bytecode"]["opcodes"].as_array().expect("opcodes");
    assert_eq!(
        compile["bytecode"]["instruction_count"].as_u64().unwrap(),
        opcodes.len() as u64
    );
    assert_eq!(compile["bytecode"]["ast_hash"].as_str().unwrap().len(), 64);
    assert_eq!(compile["bytecode"]["qir_hash"].as_str().unwrap().len(), 64);
    assert_eq!(
        compile["bytecode"]["opcodes_hash"].as_str().unwrap().len(),
        64
    );
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "BIND" && op["value"] == "question"));
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "PACK" && op["value"] == "evidence<-question"));
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "STRICT_ANSWER" && op["value"] == "answer<-evidence"));
    assert!(opcodes
        .iter()
        .any(|op| op["op"] == "EMIT" && op["value"] == "answer"));
    assert!(compile["bytecode"]["ast"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["kind"] == "resolver-step" && node["name"] == "evidence"));
    assert!(compile["bytecode"]["qir"]
        .as_array()
        .unwrap()
        .iter()
        .any(|instruction| instruction["op"] == "CALL_STRICT_ANSWER"
            && instruction["target"] == "answer"));

    fs::write(
        &qorx_file,
        r#"QORX 1
let question = "production gate routed provider evidence"
pack evidence from question budget 600
cache evidence key question ttl 3600
strict answer from evidence limit 1
emit answer
"#,
    )
    .expect("write cached qorx file");
    let compile_with_cache = qorx(
        &[
            "qorx-compile",
            qorx_file.to_str().unwrap(),
            "--out",
            bytecode_file.to_str().unwrap(),
        ],
        &qorx_home,
    );
    let cached_opcodes = compile_with_cache["bytecode"]["opcodes"]
        .as_array()
        .expect("opcodes");
    assert!(cached_opcodes
        .iter()
        .any(|op| op["op"] == "CACHE_BIND" && op["value"] == "evidence<-question"));

    let run = qorx(&["qorx", bytecode_file.to_str().unwrap()], &qorx_home);

    assert_eq!(run["source_kind"], "qorxb");
    assert_eq!(run["program"]["mode"], "program");
    assert_eq!(run["execution"]["output"]["coverage"], "supported");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn qorx_prompt_emits_third_party_tool_contract() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    seed_qorx_index(&qorx_home);
    let qorx_file = root.join("strict.qorx");
    fs::write(
        &qorx_file,
        "mode: strict-answer\nask: production gate routed provider evidence\nlimit: 1\n",
    )
    .expect("write qorx file");

    let report = qorx(&["qorx-prompt", qorx_file.to_str().unwrap()], &qorx_home);

    assert_eq!(report["schema"], "qorx.prompt.v1");
    assert_eq!(report["tool"]["name"], "qorx.resolve");
    assert_eq!(report["tool"]["input_schema"]["required"][0], "goal_hash");
    assert!(report["prompt_block"]
        .as_str()
        .unwrap()
        .contains("QORX_CALL qorx://"));
    assert!(report["prompt_block"]
        .as_str()
        .unwrap()
        .contains("call qorx.resolve"));
    assert_eq!(report["local_only"], true);

    let _ = fs::remove_dir_all(&root);
}
