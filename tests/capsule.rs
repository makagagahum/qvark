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
        "qorx-capsule-test-{}-{suffix}-{sequence}",
        std::process::id()
    ))
}

fn run_json(args: &[&str], qorx_home: &Path, aim_path: Option<&Path>) -> serde_json::Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_qorx"));
    command.args(args).env("QORX_HOME", qorx_home);
    if let Some(path) = aim_path {
        command.env("QORX_AIM_PATH", path);
    }
    let output = command
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

fn run_text(args: &[&str], qorx_home: &Path) -> String {
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
    String::from_utf8(output.stdout).expect("stdout is utf8")
}

#[test]
fn capsule_points_to_folder_memory_and_aim_with_tiny_prompt() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    let brain = root.join("brain");
    let notes = brain.join("notes");
    let secrets = brain.join("secrets");
    fs::create_dir_all(&notes).expect("create notes");
    fs::create_dir_all(&secrets).expect("create secrets");
    fs::create_dir_all(&qorx_home).expect("create qorx home");
    fs::write(
        notes.join("rag.md"),
        "The RAG brain capsule keeps exact evidence local while the model sees a tiny pointer for B2C savings.\nThe capsule supports project folders, note vaults, and exported database folders.",
    )
    .expect("write rag note");
    fs::write(
        secrets.join("api-key.md"),
        "superhidden credential should never appear in the default capsule index",
    )
    .expect("write secret note");
    let aim_path = root.join("memory.aim");
    fs::write(
        &aim_path,
        br#"AIMTTT{"chunks":7,"files":3,"mode":"local-sidecar","source":"capsule-test","workspace_root":"C:\\capsule-brain","tensor_dim":1536,"type":"qorx_aim_sidecar","version":"2026.1"}payload"#,
    )
    .expect("write aim");

    let memory = run_json(
        &[
            "memory",
            "create",
            "decision",
            "B2C capsule mode includes Marvin memory as local indexed evidence only.",
        ],
        &qorx_home,
        None,
    );
    assert_eq!(memory["schema"], "qorx.memory.v1");

    let capsule = run_json(
        &[
            "capsule",
            "create",
            brain.to_str().unwrap(),
            "--include-memory",
            "--include-aim",
            "--max-files",
            "64",
        ],
        &qorx_home,
        Some(&aim_path),
    );
    assert_eq!(capsule["schema"], "qorx.capsule.v1");
    assert!(capsule["handle"].as_str().unwrap().starts_with("qorx://c/"));
    assert!(capsule["visible_tokens"].as_u64().unwrap() < 90);
    assert!(
        capsule["indexed_tokens"].as_u64().unwrap() > capsule["visible_tokens"].as_u64().unwrap()
    );
    assert!(capsule["context_reduction_x"].as_f64().unwrap() > 1.0);
    assert_eq!(capsule["sources"].as_array().unwrap().len(), 3);
    assert!(!capsule["prompt_block"]
        .as_str()
        .unwrap()
        .contains("superhidden credential"));
    assert!(!serde_json::to_string(&capsule)
        .unwrap()
        .contains("superhidden credential"));

    let prompt_block = run_text(&["capsule", "session", "--block"], &qorx_home);
    assert!(prompt_block.starts_with("QORX_CAPSULE qorx://c/"));
    assert!(prompt_block.contains("est=char4"));
    assert!(prompt_block.contains("b2c=accounting"));
    assert!(!prompt_block.contains("RAG brain capsule keeps exact evidence"));

    let supported = run_json(
        &["capsule", "strict-answer", "rag brain capsule b2c pointer"],
        &qorx_home,
        None,
    );
    assert_eq!(supported["schema"], "qorx.capsule.strict-answer.v1");
    assert_ne!(supported["answer"]["coverage"], "not_found");
    assert!(supported["answer"]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["path"].as_str().unwrap().contains("rag.md")));

    let memory_supported = run_json(
        &["capsule", "strict-answer", "marvin memory b2c capsule"],
        &qorx_home,
        None,
    );
    assert_ne!(memory_supported["answer"]["coverage"], "not_found");
    assert!(memory_supported["answer"]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["path"].as_str().unwrap().starts_with("qorx-memory/")));

    let aim_supported = run_json(
        &["capsule", "strict-answer", "local sidecar capsule-test"],
        &qorx_home,
        None,
    );
    assert_ne!(aim_supported["answer"]["coverage"], "not_found");
    assert!(aim_supported["answer"]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["path"].as_str().unwrap() == "qorx-sidecar/metadata.json"));

    let missing = run_json(
        &["capsule", "strict-answer", "superhidden credential"],
        &qorx_home,
        None,
    );
    assert_eq!(missing["answer"]["coverage"], "not_found");
    assert_eq!(missing["answer"]["answer"], "");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bootstrap_is_refused_in_community_edition() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    let project = root.join("project");
    fs::create_dir_all(&qorx_home).expect("create qorx home");
    fs::create_dir_all(&project).expect("create project");
    fs::write(
        project.join("README.md"),
        "Qorx CE keeps bootstrap activation outside the public command surface.",
    )
    .expect("write project readme");

    let output = Command::new(env!("CARGO_BIN_EXE_qorx"))
        .args([
            "bootstrap",
            "--json",
            "--no-integrations",
            "--path",
            project.to_str().unwrap(),
        ])
        .current_dir(&project)
        .env("QORX_HOME", &qorx_home)
        .output()
        .expect("run bootstrap");
    assert!(
        !output.status.success(),
        "bootstrap unexpectedly succeeded: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not included in Qorx Community Edition"));
    assert!(stderr.contains("Qorx Local Pro"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn capsule_handle_is_stable_for_same_folder_memory_and_aim() {
    let root = unique_temp_dir();
    let qorx_home = root.join("qorx-home");
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project");
    fs::create_dir_all(&qorx_home).expect("create qorx home");
    fs::write(
        project.join("notes.md"),
        "Stable capsule handles are required so warmed cache and pointer reuse can work.",
    )
    .expect("write notes");
    let aim_path = root.join("memory.aim");
    fs::write(
        &aim_path,
        br#"AIMTTT{"chunks":1,"files":1,"mode":"local-sidecar","source":"stable-capsule","workspace_root":"C:\\stable","tensor_dim":128,"type":"qorx_aim_sidecar","version":"2026.1"}payload"#,
    )
    .expect("write aim");

    let _ = run_json(
        &[
            "memory",
            "create",
            "decision",
            "stable capsule memory must hash deterministically",
        ],
        &qorx_home,
        None,
    );

    let first = run_json(
        &[
            "capsule",
            "create",
            project.to_str().unwrap(),
            "--include-memory",
            "--include-aim",
        ],
        &qorx_home,
        Some(&aim_path),
    );
    let second = run_json(
        &[
            "capsule",
            "create",
            project.to_str().unwrap(),
            "--include-memory",
            "--include-aim",
        ],
        &qorx_home,
        Some(&aim_path),
    );

    assert_eq!(first["handle"], second["handle"]);
    assert_eq!(first["index_sha256"], second["index_sha256"]);

    let _ = fs::remove_dir_all(&root);
}
