use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    cache_plan,
    compression::estimate_tokens,
    impact,
    index::{self, RepoIndex},
    proto_store, session, squeeze, truth,
};

const PROGRAM_BOUNDARY: &str = "Qorx (.qorx) is an AI-native context language. It resolves through local index, capsule, session, cache, and evidence tools instead of embedding bulk context in the file.";
const RUN_BOUNDARY: &str = "A .qorx source file or .qorxb bytecode file is a tiny Qorx handle program over local resolver state. It can be lossless for Qorx-known handles and indexed evidence, but it is not universal compression of unknown arbitrary data.";
const PROMPT_BOUNDARY: &str = "Third-party models do not understand Qorx natively. They must call qorx.resolve, MCP, or a Qorx proxy; otherwise the prompt block is only a compact hint.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxDirective {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxProgram {
    pub schema: String,
    pub language: String,
    pub extension: String,
    pub version: String,
    pub mode: String,
    pub goal: String,
    pub handle: Option<String>,
    pub budget_tokens: u64,
    pub limit: usize,
    #[serde(default)]
    pub imports: Vec<QorxImport>,
    #[serde(default)]
    pub bindings: Vec<QorxBinding>,
    #[serde(default)]
    pub steps: Vec<QorxStep>,
    #[serde(default)]
    pub emit: Option<String>,
    #[serde(default)]
    pub branches: Vec<QorxBranch>,
    #[serde(default)]
    pub assertions: Vec<QorxAssertion>,
    #[serde(default)]
    pub cache_policies: Vec<QorxCachePolicy>,
    pub directives: Vec<QorxDirective>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxImport {
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxBinding {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxStep {
    pub name: String,
    pub op: String,
    pub source: String,
    pub budget_tokens: u64,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxAssertion {
    pub predicate: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxBranch {
    pub predicate: String,
    pub target: String,
    pub then_emit: String,
    pub else_emit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxCachePolicy {
    pub target: String,
    pub key_source: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QorxOpcode {
    pub op: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QorxStackOp {
    pub word: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QorxAstNode {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub else_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QorxIrInstruction {
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QorxBytecode {
    pub schema: String,
    pub language: String,
    pub extension: String,
    pub version: String,
    pub program: QorxProgram,
    pub opcodes: Vec<QorxOpcode>,
    #[serde(default)]
    pub qstk: Vec<QorxStackOp>,
    #[serde(default)]
    pub ast: Vec<QorxAstNode>,
    #[serde(default)]
    pub qir: Vec<QorxIrInstruction>,
    pub source_tokens: u64,
    pub goal_hash: String,
    #[serde(default)]
    pub program_hash: String,
    #[serde(default)]
    pub ast_hash: String,
    #[serde(default)]
    pub qir_hash: String,
    #[serde(default)]
    pub opcodes_hash: String,
    #[serde(default)]
    pub qstk_hash: String,
    #[serde(default)]
    pub instruction_count: usize,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxRunReport {
    pub schema: String,
    pub language: String,
    pub extension: String,
    pub file: String,
    pub source_kind: String,
    pub visible_tokens: u64,
    pub bytecode_hash: Option<String>,
    pub bytecode_bytes: Option<u64>,
    pub local_only: bool,
    pub provider_calls: u64,
    pub program: QorxProgram,
    pub execution: Value,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxCompileReport {
    pub schema: String,
    pub language: String,
    pub input: String,
    pub output: String,
    pub source_tokens: u64,
    pub bytecode_hash: String,
    pub bytecode_bytes: u64,
    pub bytecode: QorxBytecode,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxDiagnostic {
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxCheckReport {
    pub schema: String,
    pub language: String,
    pub input: String,
    pub valid: bool,
    pub diagnostics: Vec<QorxDiagnostic>,
    pub ast: Vec<QorxAstNode>,
    pub qir: Vec<QorxIrInstruction>,
    pub program: Option<QorxProgram>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxInspectReport {
    pub schema: String,
    pub language: String,
    pub file: String,
    pub source_kind: String,
    pub visible_tokens: u64,
    pub bytecode_hash: Option<String>,
    pub bytecode_bytes: Option<u64>,
    pub program_hash: String,
    pub goal_hash: String,
    pub ast_hash: String,
    pub qir_hash: String,
    pub opcodes_hash: String,
    pub qstk_hash: String,
    pub instruction_count: usize,
    pub opcodes: Vec<QorxOpcode>,
    pub qstk: Vec<QorxStackOp>,
    pub ast: Vec<QorxAstNode>,
    pub qir: Vec<QorxIrInstruction>,
    pub program: QorxProgram,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxToolContract {
    pub name: String,
    pub purpose: String,
    pub input_schema: Value,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QorxPromptReport {
    pub schema: String,
    pub language: String,
    pub extension: String,
    pub file: String,
    pub source_kind: String,
    pub handle: String,
    pub prompt_block: String,
    pub prompt_tokens: u64,
    pub local_only: bool,
    pub provider_calls: u64,
    pub tool: QorxToolContract,
    pub program_hash: String,
    pub goal_hash: String,
    pub program: QorxProgram,
    pub boundary: String,
}

#[derive(Debug, Clone)]
struct LoadedQorx {
    program: QorxProgram,
    source_kind: String,
    visible_tokens: u64,
    bytecode_hash: Option<String>,
    bytecode_bytes: Option<u64>,
    source_tokens: u64,
    bytecode: Option<QorxBytecode>,
}

#[derive(Debug, Clone)]
struct QorxDraft {
    version: String,
    mode: Option<String>,
    goal: Option<String>,
    ask: Option<String>,
    prompt: Option<String>,
    handle: Option<String>,
    budget_tokens: u64,
    limit: usize,
    imports: Vec<QorxImport>,
    bindings: Vec<QorxBinding>,
    steps: Vec<QorxStep>,
    emit: Option<String>,
    branches: Vec<QorxBranch>,
    assertions: Vec<QorxAssertion>,
    cache_policies: Vec<QorxCachePolicy>,
    directives: Vec<QorxDirective>,
}

impl Default for QorxDraft {
    fn default() -> Self {
        Self {
            version: "1".to_string(),
            mode: None,
            goal: None,
            ask: None,
            prompt: None,
            handle: None,
            budget_tokens: 900,
            limit: 2,
            imports: Vec::new(),
            bindings: Vec::new(),
            steps: Vec::new(),
            emit: None,
            branches: Vec::new(),
            assertions: Vec::new(),
            cache_policies: Vec::new(),
            directives: Vec::new(),
        }
    }
}

pub fn run_file(path: &Path, index: &RepoIndex) -> Result<QorxRunReport> {
    let loaded = load_program(path)?;
    let execution = execute_program(&loaded.program, index)?;
    Ok(QorxRunReport {
        schema: "qorx.run.v1".to_string(),
        language: "qorx".to_string(),
        extension: loaded.program.extension.clone(),
        file: path.display().to_string(),
        source_kind: loaded.source_kind,
        visible_tokens: loaded.visible_tokens,
        bytecode_hash: loaded.bytecode_hash,
        bytecode_bytes: loaded.bytecode_bytes,
        local_only: true,
        provider_calls: 0,
        program: loaded.program,
        execution,
        boundary: RUN_BOUNDARY.to_string(),
    })
}

pub fn check_file(input: &Path) -> Result<QorxCheckReport> {
    require_source_extension(input)?;
    let source = fs::read_to_string(input)
        .with_context(|| format!("could not read Qorx source {}", input.display()))?;
    let parsed = match parse_program(&source) {
        Ok(program) => program,
        Err(error) => {
            return Ok(QorxCheckReport {
                schema: "qorx.check.v1".to_string(),
                language: "qorx".to_string(),
                input: input.display().to_string(),
                valid: false,
                diagnostics: vec![diagnostic(error.to_string())],
                ast: Vec::new(),
                qir: Vec::new(),
                program: None,
                boundary: "qorx-check parses source and runs compiler semantic checks without executing resolver calls.".to_string(),
            });
        }
    };
    let ast = ast_from_program(&parsed);
    match validate_program(&parsed) {
        Ok(()) => Ok(QorxCheckReport {
            schema: "qorx.check.v1".to_string(),
            language: "qorx".to_string(),
            input: input.display().to_string(),
            valid: true,
            diagnostics: Vec::new(),
            qir: qir_from_program(&parsed),
            ast,
            program: Some(parsed),
            boundary: "qorx-check parses source and runs compiler semantic checks without executing resolver calls.".to_string(),
        }),
        Err(error) => Ok(QorxCheckReport {
            schema: "qorx.check.v1".to_string(),
            language: "qorx".to_string(),
            input: input.display().to_string(),
            valid: false,
            diagnostics: vec![diagnostic(error.to_string())],
            ast,
            qir: Vec::new(),
            program: Some(parsed),
            boundary: "qorx-check parses source and runs compiler semantic checks without executing resolver calls.".to_string(),
        }),
    }
}

pub fn compile_file(input: &Path, output: Option<&Path>) -> Result<QorxCompileReport> {
    require_source_extension(input)?;
    let source = fs::read_to_string(input)
        .with_context(|| format!("could not read Qorx source {}", input.display()))?;
    let program = parse_checked_program(&source)?;
    let source_tokens = estimate_tokens(&source);
    let bytecode = bytecode_from_program(program, source_tokens)?;
    let output_path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("qorxb"));

    proto_store::save(&output_path, &bytecode)?;
    let encoded = fs::read(&output_path)
        .with_context(|| format!("could not read compiled bytecode {}", output_path.display()))?;

    Ok(QorxCompileReport {
        schema: "qorx.compile.v1".to_string(),
        language: "qorx".to_string(),
        input: input.display().to_string(),
        output: output_path.display().to_string(),
        source_tokens,
        bytecode_hash: hex_sha256(&encoded),
        bytecode_bytes: encoded.len() as u64,
        bytecode,
        boundary: "The .qorxb file is Qorx protobuf-envelope bytecode. It is compact and exact for the parsed program, but it still resolves meaning through local Qorx state.".to_string(),
    })
}

pub fn inspect_file(path: &Path) -> Result<QorxInspectReport> {
    let loaded = load_program(path)?;
    let program_hash = program_hash(&loaded.program)?;
    let goal_hash = hex_sha256(loaded.program.goal.as_bytes());
    let bytecode = match &loaded.bytecode {
        Some(bytecode) => bytecode.clone(),
        None => bytecode_from_program(loaded.program.clone(), loaded.source_tokens)?,
    };
    Ok(QorxInspectReport {
        schema: "qorx.inspect.v1".to_string(),
        language: "qorx".to_string(),
        file: path.display().to_string(),
        source_kind: loaded.source_kind,
        visible_tokens: loaded.visible_tokens,
        bytecode_hash: loaded.bytecode_hash,
        bytecode_bytes: loaded.bytecode_bytes,
        program_hash,
        goal_hash,
        ast_hash: bytecode.ast_hash.clone(),
        qir_hash: bytecode.qir_hash.clone(),
        opcodes_hash: bytecode.opcodes_hash.clone(),
        qstk_hash: bytecode.qstk_hash.clone(),
        instruction_count: bytecode.instruction_count,
        opcodes: bytecode.opcodes,
        qstk: bytecode.qstk,
        ast: bytecode.ast,
        qir: bytecode.qir,
        program: loaded.program,
        boundary: RUN_BOUNDARY.to_string(),
    })
}

pub fn prompt_file(path: &Path) -> Result<QorxPromptReport> {
    let loaded = load_program(path)?;
    let program_hash = program_hash(&loaded.program)?;
    let goal_hash = hex_sha256(loaded.program.goal.as_bytes());
    let handle = format!("qorx://{}", short_hash(&program_hash));
    let prompt_block = format!(
        "QORX_CALL {handle}\nlanguage=qorx ext=.qorx/.qorxb mode={} goal_hash={goal_hash}\ntool=qorx.resolve; rule=call qorx.resolve; do not guess; local_idx stays local; resolve with Qorx.",
        loaded.program.mode
    );
    let tool = QorxToolContract {
        name: "qorx.resolve".to_string(),
        purpose: "Resolve a Qorx program or qorx:// handle through local Qorx evidence, session, capsule, cache, and index state.".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["goal_hash", "program"],
            "properties": {
                "goal_hash": {
                    "type": "string",
                    "description": "sha256 of the Qorx goal"
                },
                "program": {
                    "type": "string",
                    "description": ".qorx source, .qorxb bytecode handle, or qorx:// pointer"
                },
                "mode": {
                    "type": "string",
                    "enum": ["agent", "strict-answer", "pack", "squeeze", "map", "cache-plan", "session"]
                },
                "budget_tokens": {
                    "type": "integer",
                    "minimum": 128
                }
            },
            "additionalProperties": false
        }),
        rule: "call qorx.resolve before answering; never infer hidden local evidence from the compact handle alone".to_string(),
    };

    Ok(QorxPromptReport {
        schema: "qorx.prompt.v1".to_string(),
        language: "qorx".to_string(),
        extension: loaded.program.extension.clone(),
        file: path.display().to_string(),
        source_kind: loaded.source_kind,
        handle,
        prompt_tokens: estimate_tokens(&prompt_block),
        prompt_block,
        local_only: true,
        provider_calls: 0,
        tool,
        program_hash,
        goal_hash,
        program: loaded.program,
        boundary: PROMPT_BOUNDARY.to_string(),
    })
}

fn load_program(path: &Path) -> Result<LoadedQorx> {
    match normalized_extension(path).as_deref() {
        Some("qorx") => {
            let source = fs::read_to_string(path)
                .with_context(|| format!("could not read Qorx source {}", path.display()))?;
            let program = parse_checked_program(&source)?;
            let source_tokens = estimate_tokens(&source);
            Ok(LoadedQorx {
                program,
                source_kind: normalized_extension(path).unwrap_or_else(|| "qorx".to_string()),
                visible_tokens: source_tokens,
                bytecode_hash: None,
                bytecode_bytes: None,
                source_tokens,
                bytecode: None,
            })
        }
        Some("qorxb") => {
            let encoded = fs::read(path)
                .with_context(|| format!("could not read Qorx bytecode {}", path.display()))?;
            let bytecode: QorxBytecode = proto_store::load_required(path, &[])?;
            validate_bytecode(&bytecode, path)?;
            let program = bytecode.program.clone();
            let source_tokens = bytecode.source_tokens;
            Ok(LoadedQorx {
                program,
                source_kind: "qorxb".to_string(),
                visible_tokens: 0,
                bytecode_hash: Some(hex_sha256(&encoded)),
                bytecode_bytes: Some(encoded.len() as u64),
                source_tokens,
                bytecode: Some(bytecode),
            })
        }
        _ => Err(anyhow!(
            "Qorx programs must use .qorx source or .qorxb bytecode: {}",
            path.display()
        )),
    }
}

fn parse_program(source: &str) -> Result<QorxProgram> {
    let mut draft = QorxDraft::default();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if let Some(version) = line.strip_prefix("QORX ") {
            draft.version = version.trim().to_string();
            continue;
        }
        if let Some(statement) = parse_statement(line, draft.budget_tokens, draft.limit)? {
            apply_statement(&mut draft, statement)?;
            continue;
        }
        let Some((key, value)) = parse_directive(line) else {
            return Err(anyhow!("unsupported Qorx directive: {line}"));
        };
        apply_directive(&mut draft, &key, &value)?;
    }

    let has_language_program = draft_has_language_items(&draft);
    let mode = draft.mode.clone().unwrap_or_else(|| {
        if has_language_program {
            "program".to_string()
        } else {
            inferred_mode(&draft).to_string()
        }
    });
    let mode = normalize_mode(&mode)?;
    let goal = draft
        .goal
        .clone()
        .or_else(|| draft.ask.clone())
        .or_else(|| draft.prompt.clone())
        .or_else(|| inferred_program_goal(&draft))
        .or_else(|| {
            has_language_program
                .then(|| draft.steps.first().map(|step| step.source.clone()))
                .flatten()
        })
        .ok_or_else(|| anyhow!("Qorx program needs a goal:, ask:, or prompt: directive"))?;

    Ok(QorxProgram {
        schema: "qorx.program.v1".to_string(),
        language: "qorx".to_string(),
        extension: ".qorx".to_string(),
        version: draft.version,
        mode,
        goal,
        handle: draft.handle,
        budget_tokens: draft.budget_tokens.clamp(128, 20_000),
        limit: draft.limit.max(1),
        imports: draft.imports,
        bindings: draft.bindings,
        steps: draft.steps,
        emit: draft.emit,
        branches: draft.branches,
        assertions: draft.assertions,
        cache_policies: draft.cache_policies,
        directives: draft.directives,
        boundary: PROGRAM_BOUNDARY.to_string(),
    })
}

fn draft_has_language_items(draft: &QorxDraft) -> bool {
    !draft.imports.is_empty()
        || !draft.bindings.is_empty()
        || !draft.steps.is_empty()
        || draft.emit.is_some()
        || !draft.branches.is_empty()
        || !draft.assertions.is_empty()
        || !draft.cache_policies.is_empty()
}

enum ParsedStatement {
    Import(QorxImport),
    Binding(QorxBinding),
    Step(QorxStep),
    Emit(String),
    Branch(QorxBranch),
    Assertion(QorxAssertion),
    CachePolicy(QorxCachePolicy),
}

fn parse_statement(
    line: &str,
    default_budget_tokens: u64,
    default_limit: usize,
) -> Result<Option<ParsedStatement>> {
    if let Some(rest) = line.strip_prefix("use ") {
        let import = parse_import(rest.trim())?;
        return Ok(Some(ParsedStatement::Import(import)));
    }

    if let Some(rest) = line.strip_prefix("let ") {
        let (name, value) = rest
            .split_once('=')
            .ok_or_else(|| anyhow!("Qorx let statement must use `let name = \"value\"`"))?;
        let name = normalize_identifier(name.trim())?;
        let value = parse_string_literal(value.trim())?;
        return Ok(Some(ParsedStatement::Binding(QorxBinding { name, value })));
    }

    if let Some(rest) = line.strip_prefix("emit ") {
        let target = normalize_identifier(rest.trim())?;
        return Ok(Some(ParsedStatement::Emit(target)));
    }

    if let Some(rest) = line.strip_prefix("if ") {
        let branch = parse_branch(rest.trim())?;
        return Ok(Some(ParsedStatement::Branch(branch)));
    }

    if let Some(rest) = line.strip_prefix("assert ") {
        let assertion = parse_assertion(rest.trim())?;
        return Ok(Some(ParsedStatement::Assertion(assertion)));
    }

    if let Some(rest) = line.strip_prefix("cache ") {
        let policy = parse_cache_policy(rest.trim())?;
        return Ok(Some(ParsedStatement::CachePolicy(policy)));
    }

    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() >= 4 {
        let op = normalize_statement_op(tokens[0])?;
        if let Some(op) = op {
            if tokens[2] != "from" {
                return Err(anyhow!(
                    "Qorx resolver step must use `{op} name from source`"
                ));
            }
            let name = normalize_identifier(tokens[1])?;
            let source = normalize_identifier(tokens[3])?;
            let mut budget_tokens = default_budget_tokens;
            let mut limit = default_limit.max(1);
            let mut index = 4;
            while index < tokens.len() {
                match tokens[index] {
                    "budget" | "budget-tokens" => {
                        let value = tokens
                            .get(index + 1)
                            .ok_or_else(|| anyhow!("budget needs an integer value"))?;
                        budget_tokens = value
                            .parse::<u64>()
                            .map_err(|_| anyhow!("budget must be an integer token estimate"))?;
                        index += 2;
                    }
                    "limit" => {
                        let value = tokens
                            .get(index + 1)
                            .ok_or_else(|| anyhow!("limit needs an integer value"))?;
                        limit = value
                            .parse::<usize>()
                            .map_err(|_| anyhow!("limit must be an integer"))?
                            .max(1);
                        index += 2;
                    }
                    other => {
                        return Err(anyhow!("unknown Qorx step option `{other}`"));
                    }
                }
            }
            return Ok(Some(ParsedStatement::Step(QorxStep {
                name,
                op,
                source,
                budget_tokens: budget_tokens.clamp(128, 20_000),
                limit,
            })));
        }
    }

    Ok(None)
}

fn parse_import(value: &str) -> Result<QorxImport> {
    let (module, alias) = if let Some((module, alias)) = value.split_once(" as ") {
        (module.trim(), Some(normalize_identifier(alias.trim())?))
    } else {
        (value.trim(), None)
    };
    Ok(QorxImport {
        module: normalize_module_name(module)?,
        alias,
    })
}

fn parse_branch(value: &str) -> Result<QorxBranch> {
    let (condition, rest) = value.split_once(" then emit ").ok_or_else(|| {
        anyhow!("Qorx branch must use `if supported(name) then emit a else emit b`")
    })?;
    let (then_emit, else_emit) = rest.split_once(" else emit ").ok_or_else(|| {
        anyhow!("Qorx branch must use `if supported(name) then emit a else emit b`")
    })?;
    let condition = parse_assertion(condition.trim())?;
    Ok(QorxBranch {
        predicate: condition.predicate,
        target: condition.target,
        then_emit: normalize_identifier(then_emit.trim())?,
        else_emit: normalize_identifier(else_emit.trim())?,
    })
}

fn normalize_module_name(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Qorx import module cannot be empty"));
    }
    if !trimmed
        .chars()
        .all(|ch| ch == '.' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
    {
        return Err(anyhow!(
            "Qorx import module `{trimmed}` may only contain letters, numbers, _, -, or ."
        ));
    }
    Ok(trimmed.to_string())
}

fn parse_cache_policy(value: &str) -> Result<QorxCachePolicy> {
    let tokens: Vec<&str> = value.split_whitespace().collect();
    if tokens.len() != 5 || tokens[1] != "key" || tokens[3] != "ttl" {
        return Err(anyhow!(
            "Qorx cache policy must use `cache target key source ttl seconds`"
        ));
    }
    let ttl_seconds = tokens[4]
        .parse::<u64>()
        .map_err(|_| anyhow!("cache ttl must be an integer number of seconds"))?;
    if ttl_seconds == 0 {
        return Err(anyhow!("cache ttl must be greater than zero"));
    }
    Ok(QorxCachePolicy {
        target: normalize_identifier(tokens[0])?,
        key_source: normalize_identifier(tokens[2])?,
        ttl_seconds,
    })
}

fn parse_assertion(value: &str) -> Result<QorxAssertion> {
    let trimmed = value.trim();
    let target = if let Some(inner) = trimmed
        .strip_prefix("supported(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        inner
    } else if let Some(rest) = trimmed.strip_prefix("supported ") {
        rest
    } else {
        return Err(anyhow!(
            "Qorx assert supports `assert supported(name)` in this release"
        ));
    };
    Ok(QorxAssertion {
        predicate: "supported".to_string(),
        target: normalize_identifier(target.trim())?,
    })
}

fn normalize_statement_op(token: &str) -> Result<Option<String>> {
    let op = match token.trim().to_lowercase().replace('_', "-").as_str() {
        "pack" => Some("pack"),
        "strict" | "strict-answer" | "answer" => Some("strict-answer"),
        "squeeze" | "extract" => Some("squeeze"),
        "map" | "repo-map" => Some("map"),
        "session" => Some("session"),
        "cache" | "cache-plan" => Some("cache-plan"),
        _ => None,
    };
    Ok(op.map(str::to_string))
}

fn normalize_identifier(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return Err(anyhow!("Qorx identifier cannot be empty"));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(anyhow!(
            "Qorx identifier `{trimmed}` must start with a letter or _"
        ));
    }
    if !chars.all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()) {
        return Err(anyhow!(
            "Qorx identifier `{trimmed}` may only contain letters, numbers, _, or -"
        ));
    }
    Ok(trimmed.to_string())
}

fn parse_string_literal(value: &str) -> Result<String> {
    let Some(inner) = value
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
    else {
        return Err(anyhow!(
            "Qorx string literal must be wrapped in double quotes"
        ));
    };
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let escaped = chars
            .next()
            .ok_or_else(|| anyhow!("Qorx string literal ends with an escape"))?;
        match escaped {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            'n' => out.push('\n'),
            't' => out.push('\t'),
            other => return Err(anyhow!("unsupported Qorx string escape `\\{other}`")),
        }
    }
    Ok(out)
}

fn parse_checked_program(source: &str) -> Result<QorxProgram> {
    let program = parse_program(source)?;
    validate_program(&program)?;
    Ok(program)
}

fn validate_program(program: &QorxProgram) -> Result<()> {
    if program.steps.is_empty() && (!program.assertions.is_empty() || !program.branches.is_empty())
    {
        return Err(anyhow!(
            "Qorx assertions and branches need a resolver step result to inspect"
        ));
    }

    let mut symbols: HashMap<String, &'static str> = HashMap::new();
    for import in &program.imports {
        validate_import(import)?;
    }
    for binding in &program.bindings {
        symbols.insert(binding.name.clone(), "binding");
    }

    for step in &program.steps {
        if !symbols.contains_key(&step.source) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used as source for `{}`",
                step.source,
                step.name
            ));
        }
        symbols.insert(step.name.clone(), "step");
    }

    for policy in &program.cache_policies {
        if !symbols.contains_key(&policy.target) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used as cache target",
                policy.target
            ));
        }
        if !symbols.contains_key(&policy.key_source) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used as cache key",
                policy.key_source
            ));
        }
    }

    for assertion in &program.assertions {
        if assertion.predicate != "supported" {
            return Err(anyhow!(
                "unsupported Qorx assertion predicate `{}`",
                assertion.predicate
            ));
        }
        if !symbols.contains_key(&assertion.target) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used by assert supported",
                assertion.target
            ));
        }
    }

    for branch in &program.branches {
        if branch.predicate != "supported" {
            return Err(anyhow!(
                "unsupported Qorx branch predicate `{}`",
                branch.predicate
            ));
        }
        if !symbols.contains_key(&branch.target) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used by if supported",
                branch.target
            ));
        }
        if !symbols.contains_key(&branch.then_emit) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used by branch then emit",
                branch.then_emit
            ));
        }
        if !symbols.contains_key(&branch.else_emit) {
            return Err(anyhow!(
                "undefined Qorx symbol `{}` used by branch else emit",
                branch.else_emit
            ));
        }
    }

    if let Some(emit) = &program.emit {
        if !symbols.contains_key(emit) {
            return Err(anyhow!("undefined Qorx symbol `{emit}` used by emit"));
        }
    }

    Ok(())
}

fn validate_import(import: &QorxImport) -> Result<()> {
    const STD_MODULES: &[&str] = &[
        "std.ctx",
        "std.evidence",
        "std.cache",
        "std.branch",
        "std.session",
        "std.qosm",
        "std.qshf",
    ];
    if !STD_MODULES.contains(&import.module.as_str()) {
        return Err(anyhow!("unknown Qorx std module `{}`", import.module));
    }
    Ok(())
}

fn diagnostic(message: String) -> QorxDiagnostic {
    QorxDiagnostic {
        severity: "error".to_string(),
        message,
    }
}

fn ast_from_program(program: &QorxProgram) -> Vec<QorxAstNode> {
    let mut nodes = vec![QorxAstNode {
        kind: "version".to_string(),
        name: None,
        op: None,
        source: None,
        target: None,
        else_target: None,
        value_hash: Some(hex_sha256(program.version.as_bytes())),
        budget_tokens: None,
        limit: None,
        ttl_seconds: None,
    }];
    for import in &program.imports {
        nodes.push(QorxAstNode {
            kind: "import".to_string(),
            name: Some(import.module.clone()),
            op: Some("use".to_string()),
            source: None,
            target: import.alias.clone(),
            else_target: None,
            value_hash: Some(hex_sha256(import.module.as_bytes())),
            budget_tokens: None,
            limit: None,
            ttl_seconds: None,
        });
    }
    for binding in &program.bindings {
        nodes.push(QorxAstNode {
            kind: "binding".to_string(),
            name: Some(binding.name.clone()),
            op: None,
            source: None,
            target: None,
            else_target: None,
            value_hash: Some(hex_sha256(binding.value.as_bytes())),
            budget_tokens: None,
            limit: None,
            ttl_seconds: None,
        });
    }
    for step in &program.steps {
        nodes.push(QorxAstNode {
            kind: "resolver-step".to_string(),
            name: Some(step.name.clone()),
            op: Some(step.op.clone()),
            source: Some(step.source.clone()),
            target: None,
            else_target: None,
            value_hash: None,
            budget_tokens: Some(step.budget_tokens),
            limit: Some(step.limit),
            ttl_seconds: None,
        });
    }
    for policy in &program.cache_policies {
        nodes.push(QorxAstNode {
            kind: "cache-policy".to_string(),
            name: None,
            op: Some("cache".to_string()),
            source: Some(policy.key_source.clone()),
            target: Some(policy.target.clone()),
            else_target: None,
            value_hash: None,
            budget_tokens: None,
            limit: None,
            ttl_seconds: Some(policy.ttl_seconds),
        });
    }
    for assertion in &program.assertions {
        nodes.push(QorxAstNode {
            kind: "assert-supported".to_string(),
            name: None,
            op: Some(assertion.predicate.clone()),
            source: None,
            target: Some(assertion.target.clone()),
            else_target: None,
            value_hash: None,
            budget_tokens: None,
            limit: None,
            ttl_seconds: None,
        });
    }
    for branch in &program.branches {
        nodes.push(QorxAstNode {
            kind: "branch".to_string(),
            name: None,
            op: Some(branch.predicate.clone()),
            source: Some(branch.target.clone()),
            target: Some(branch.then_emit.clone()),
            else_target: Some(branch.else_emit.clone()),
            value_hash: None,
            budget_tokens: None,
            limit: None,
            ttl_seconds: None,
        });
    }
    if let Some(emit) = &program.emit {
        nodes.push(QorxAstNode {
            kind: "emit".to_string(),
            name: None,
            op: None,
            source: None,
            target: Some(emit.clone()),
            else_target: None,
            value_hash: None,
            budget_tokens: None,
            limit: None,
            ttl_seconds: None,
        });
    }
    nodes
}

fn qir_from_program(program: &QorxProgram) -> Vec<QorxIrInstruction> {
    let mut instructions = Vec::new();
    for import in &program.imports {
        let mut args = BTreeMap::new();
        args.insert(
            "module_sha256".to_string(),
            hex_sha256(import.module.as_bytes()),
        );
        instructions.push(QorxIrInstruction {
            op: "IMPORT_MODULE".to_string(),
            target: import.alias.clone(),
            source: Some(import.module.clone()),
            args,
        });
    }
    for binding in &program.bindings {
        let mut args = BTreeMap::new();
        args.insert(
            "value_sha256".to_string(),
            hex_sha256(binding.value.as_bytes()),
        );
        instructions.push(QorxIrInstruction {
            op: "BIND_CONST".to_string(),
            target: Some(binding.name.clone()),
            source: None,
            args,
        });
    }
    for step in &program.steps {
        let mut args = BTreeMap::new();
        args.insert("budget_tokens".to_string(), step.budget_tokens.to_string());
        args.insert("limit".to_string(), step.limit.to_string());
        instructions.push(QorxIrInstruction {
            op: qir_call_op(&step.op),
            target: Some(step.name.clone()),
            source: Some(step.source.clone()),
            args,
        });
    }
    for assertion in &program.assertions {
        instructions.push(QorxIrInstruction {
            op: "ASSERT_SUPPORTED".to_string(),
            target: Some(assertion.target.clone()),
            source: None,
            args: BTreeMap::new(),
        });
    }
    for branch in &program.branches {
        let mut args = BTreeMap::new();
        args.insert("predicate".to_string(), branch.predicate.clone());
        args.insert("else_emit".to_string(), branch.else_emit.clone());
        instructions.push(QorxIrInstruction {
            op: "IF_SUPPORTED".to_string(),
            target: Some(branch.then_emit.clone()),
            source: Some(branch.target.clone()),
            args,
        });
    }
    for policy in &program.cache_policies {
        let mut args = BTreeMap::new();
        args.insert("ttl_seconds".to_string(), policy.ttl_seconds.to_string());
        instructions.push(QorxIrInstruction {
            op: "CACHE_BIND".to_string(),
            target: Some(policy.target.clone()),
            source: Some(policy.key_source.clone()),
            args,
        });
    }
    if let Some(emit) = &program.emit {
        instructions.push(QorxIrInstruction {
            op: "EMIT".to_string(),
            target: Some(emit.clone()),
            source: None,
            args: BTreeMap::new(),
        });
    }
    instructions
}

fn qir_call_op(op: &str) -> String {
    format!("CALL_{}", statement_opcode(op))
}

fn apply_statement(draft: &mut QorxDraft, statement: ParsedStatement) -> Result<()> {
    match statement {
        ParsedStatement::Import(import) => draft.imports.push(import),
        ParsedStatement::Binding(binding) => {
            ensure_unique_name(draft, &binding.name)?;
            draft.bindings.push(binding);
        }
        ParsedStatement::Step(step) => {
            ensure_unique_name(draft, &step.name)?;
            draft.steps.push(step);
        }
        ParsedStatement::Emit(target) => draft.emit = Some(target),
        ParsedStatement::Branch(branch) => draft.branches.push(branch),
        ParsedStatement::Assertion(assertion) => draft.assertions.push(assertion),
        ParsedStatement::CachePolicy(policy) => draft.cache_policies.push(policy),
    }
    Ok(())
}

fn ensure_unique_name(draft: &QorxDraft, name: &str) -> Result<()> {
    if draft.bindings.iter().any(|binding| binding.name == name)
        || draft.steps.iter().any(|step| step.name == name)
    {
        return Err(anyhow!("duplicate Qorx symbol `{name}`"));
    }
    Ok(())
}

fn inferred_program_goal(draft: &QorxDraft) -> Option<String> {
    for step in &draft.steps {
        if let Some(binding) = draft
            .bindings
            .iter()
            .find(|binding| binding.name == step.source)
        {
            return Some(binding.value.clone());
        }
    }
    draft.bindings.first().map(|binding| binding.value.clone())
}

fn parse_directive(line: &str) -> Option<(String, String)> {
    if let Some(rest) = line.strip_prefix('@') {
        let mut parts = rest.splitn(2, char::is_whitespace);
        let key = parts.next()?.trim();
        let value = parts.next().unwrap_or_default().trim();
        return Some((key.to_string(), value.to_string()));
    }
    let (key, value) = line.split_once(':')?;
    Some((key.trim().to_string(), value.trim().to_string()))
}

fn apply_directive(draft: &mut QorxDraft, key: &str, value: &str) -> Result<()> {
    let key = key.trim().to_lowercase().replace('_', "-");
    let value = value.trim().to_string();
    match key.as_str() {
        "version" => draft.version = value.clone(),
        "mode" => draft.mode = Some(value.clone()),
        "goal" | "objective" => draft.goal = Some(value.clone()),
        "ask" | "question" => draft.ask = Some(value.clone()),
        "prompt" => draft.prompt = Some(value.clone()),
        "handle" | "session" | "capsule" => draft.handle = Some(value.clone()),
        "budget" | "budget-tokens" => {
            draft.budget_tokens = value
                .parse::<u64>()
                .map_err(|_| anyhow!("budget must be an integer token estimate"))?;
        }
        "limit" => {
            draft.limit = value
                .parse::<usize>()
                .map_err(|_| anyhow!("limit must be an integer"))?;
        }
        _ => return Err(anyhow!("unknown Qorx directive `{key}`")),
    }
    draft.directives.push(QorxDirective { key, value });
    Ok(())
}

fn inferred_mode(draft: &QorxDraft) -> &'static str {
    if draft.prompt.is_some() && draft.goal.is_none() && draft.ask.is_none() {
        "cache-plan"
    } else if draft.ask.is_some() && draft.goal.is_none() {
        "strict-answer"
    } else {
        "agent"
    }
}

fn normalize_mode(mode: &str) -> Result<String> {
    match mode.trim().to_lowercase().replace('_', "-").as_str() {
        "agent" | "execute" | "exec" | "plan" => Ok("agent".to_string()),
        "strict" | "strict-answer" | "answer" => Ok("strict-answer".to_string()),
        "pack" | "context" => Ok("pack".to_string()),
        "squeeze" | "extract" => Ok("squeeze".to_string()),
        "map" | "repo-map" => Ok("map".to_string()),
        "cache" | "cache-plan" => Ok("cache-plan".to_string()),
        "session" => Ok("session".to_string()),
        "program" | "pipeline" => Ok("program".to_string()),
        other => Err(anyhow!("unsupported Qorx mode `{other}`")),
    }
}

fn execute_program(program: &QorxProgram, index: &RepoIndex) -> Result<Value> {
    if program.mode == "program" || program_has_language_items(program) {
        return execute_language_program(program, index);
    }
    let value = match program.mode.as_str() {
        "agent" => serde_json::to_value(truth::run_agent(
            index,
            &program.goal,
            program.budget_tokens,
        ))?,
        "strict-answer" => {
            serde_json::to_value(truth::strict_answer(index, &program.goal, program.limit))?
        }
        "pack" => serde_json::to_value(index::pack_context(
            index,
            &program.goal,
            program.budget_tokens,
        ))?,
        "squeeze" => serde_json::to_value(squeeze::squeeze_context(
            index,
            &program.goal,
            program.budget_tokens,
            program.limit,
        ))?,
        "map" => serde_json::to_value(impact::map_context(
            index,
            &program.goal,
            None,
            program.budget_tokens,
        ))?,
        "cache-plan" => serde_json::to_value(cache_plan::plan_prompt(&program.goal))?,
        "session" => serde_json::to_value(session::build_session_pointer(index))?,
        _ => return Err(anyhow!("unsupported Qorx mode `{}`", program.mode)),
    };
    Ok(value)
}

fn program_has_language_items(program: &QorxProgram) -> bool {
    !program.imports.is_empty()
        || !program.bindings.is_empty()
        || !program.steps.is_empty()
        || program.emit.is_some()
        || !program.branches.is_empty()
        || !program.assertions.is_empty()
        || !program.cache_policies.is_empty()
}

fn execute_language_program(program: &QorxProgram, index: &RepoIndex) -> Result<Value> {
    let mut bindings: HashMap<String, String> = program
        .bindings
        .iter()
        .map(|binding| (binding.name.clone(), binding.value.clone()))
        .collect();
    let mut step_queries: HashMap<String, String> = HashMap::new();
    let mut values: HashMap<String, Value> = HashMap::new();
    let mut traces = Vec::new();
    let mut assertion_traces = Vec::new();
    let mut branch_traces = Vec::new();
    let mut cache_traces = Vec::new();
    let import_traces = program
        .imports
        .iter()
        .map(|import| {
            json!({
                "module": import.module,
                "alias": import.alias,
                "kind": "qorx-stdlib"
            })
        })
        .collect::<Vec<_>>();

    for binding in &program.bindings {
        values.insert(binding.name.clone(), json!({ "value": binding.value }));
    }

    for step in &program.steps {
        let query = resolve_source(&step.source, &bindings, &step_queries)?;
        let result = execute_step(step, &query, index)?;
        let schema = result
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("json")
            .to_string();
        step_queries.insert(step.name.clone(), query.clone());
        values.insert(step.name.clone(), result);
        traces.push(json!({
            "name": step.name,
            "op": step.op,
            "source": step.source,
            "query_hash": hex_sha256(query.as_bytes()),
            "budget_tokens": step.budget_tokens,
            "limit": step.limit,
            "result_schema": schema
        }));
        bindings.insert(step.name.clone(), query);
    }

    for policy in &program.cache_policies {
        let key_material = resolve_source(&policy.key_source, &bindings, &step_queries)?;
        let key = cache_policy_key(policy, &key_material);
        cache_traces.push(json!({
            "target": policy.target,
            "key_source": policy.key_source,
            "key": key,
            "ttl_seconds": policy.ttl_seconds,
            "strategy": "qorx-local-runtime-cache"
        }));
    }

    for assertion in &program.assertions {
        let value = values.get(&assertion.target).ok_or_else(|| {
            anyhow!(
                "Qorx assertion target `{}` was not produced",
                assertion.target
            )
        })?;
        let passed = match assertion.predicate.as_str() {
            "supported" => value_is_supported(value),
            other => return Err(anyhow!("unsupported Qorx assertion predicate `{other}`")),
        };
        assertion_traces.push(json!({
            "predicate": assertion.predicate,
            "target": assertion.target,
            "passed": passed
        }));
        if !passed {
            return Err(anyhow!("assert supported({}) failed", assertion.target));
        }
    }

    let mut branch_emitted = None;
    for branch in &program.branches {
        let value = values
            .get(&branch.target)
            .ok_or_else(|| anyhow!("Qorx branch target `{}` was not produced", branch.target))?;
        let passed = match branch.predicate.as_str() {
            "supported" => value_is_supported(value),
            other => return Err(anyhow!("unsupported Qorx branch predicate `{other}`")),
        };
        let emitted = if passed {
            branch.then_emit.clone()
        } else {
            branch.else_emit.clone()
        };
        branch_traces.push(json!({
            "predicate": branch.predicate,
            "condition": branch.target,
            "passed": passed,
            "then_emit": branch.then_emit,
            "else_emit": branch.else_emit,
            "taken": if passed { "then" } else { "else" },
            "emitted": emitted
        }));
        branch_emitted = Some(emitted);
    }

    let emitted = branch_emitted
        .or_else(|| program.emit.clone())
        .or_else(|| program.steps.last().map(|step| step.name.clone()))
        .ok_or_else(|| anyhow!("Qorx program needs at least one resolver step or emit target"))?;
    let output = values
        .get(&emitted)
        .cloned()
        .ok_or_else(|| anyhow!("Qorx emit target `{emitted}` was not produced"))?;

    Ok(json!({
        "schema": "qorx.program-execution.v1",
        "language": "qorx",
        "emitted": emitted,
        "imports": import_traces,
        "steps": traces,
        "cache": cache_traces,
        "assertions": assertion_traces,
        "branches": branch_traces,
        "output": output,
        "provider_calls": 0,
        "boundary": "Qorx programs execute local resolver opcodes over protobuf-backed Qorx state. They can pass compact bytecode or handles between agents, but hidden evidence still requires a resolver."
    }))
}

fn cache_policy_key(policy: &QorxCachePolicy, key_material: &str) -> String {
    let seed = format!("{}\n{}\n{}", policy.target, policy.key_source, key_material);
    format!("qrc_{}", &hex_sha256(seed.as_bytes())[..16])
}

fn value_is_supported(value: &Value) -> bool {
    if let Some(coverage) = value.get("coverage").and_then(Value::as_str) {
        return coverage == "supported";
    }
    value
        .get("evidence")
        .and_then(Value::as_array)
        .map(|evidence| !evidence.is_empty())
        .unwrap_or(false)
}

fn resolve_source(
    source: &str,
    bindings: &HashMap<String, String>,
    step_queries: &HashMap<String, String>,
) -> Result<String> {
    bindings
        .get(source)
        .or_else(|| step_queries.get(source))
        .cloned()
        .ok_or_else(|| anyhow!("unknown Qorx source `{source}`"))
}

fn execute_step(step: &QorxStep, query: &str, index: &RepoIndex) -> Result<Value> {
    let value = match step.op.as_str() {
        "pack" => serde_json::to_value(index::pack_context(index, query, step.budget_tokens))?,
        "strict-answer" => serde_json::to_value(truth::strict_answer(index, query, step.limit))?,
        "squeeze" => serde_json::to_value(squeeze::squeeze_context(
            index,
            query,
            step.budget_tokens,
            step.limit,
        ))?,
        "map" => serde_json::to_value(impact::map_context(index, query, None, step.budget_tokens))?,
        "cache-plan" => serde_json::to_value(cache_plan::plan_prompt(query))?,
        "session" => serde_json::to_value(session::build_session_pointer(index))?,
        other => return Err(anyhow!("unsupported Qorx step op `{other}`")),
    };
    Ok(value)
}

fn bytecode_from_program(program: QorxProgram, source_tokens: u64) -> Result<QorxBytecode> {
    let goal_hash = hex_sha256(program.goal.as_bytes());
    let ast = ast_from_program(&program);
    let qir = qir_from_program(&program);
    let qstk = qstk_from_program(&program, &goal_hash);
    let mut opcodes = vec![
        opcode("VERSION", &program.version),
        opcode("MODE", &program.mode),
    ];
    if let Some(handle) = &program.handle {
        opcodes.push(opcode("HANDLE", handle));
    }
    if program.mode == "program" || program_has_language_items(&program) {
        for import in &program.imports {
            match &import.alias {
                Some(alias) => opcodes.push(opcode(
                    "IMPORT_AS",
                    &format!("{} as {}", import.module, alias),
                )),
                None => opcodes.push(opcode("IMPORT", &import.module)),
            }
        }
        for binding in &program.bindings {
            opcodes.push(opcode("BIND", &binding.name));
            opcodes.push(opcode(
                "CONST_SHA256",
                &format!("{}={}", binding.name, hex_sha256(binding.value.as_bytes())),
            ));
        }
        for step in &program.steps {
            opcodes.push(opcode(
                &statement_opcode(&step.op),
                &format!("{}<-{}", step.name, step.source),
            ));
            opcodes.push(opcode(
                "STEP_BUDGET_TOKENS",
                &format!("{}={}", step.name, step.budget_tokens),
            ));
            opcodes.push(opcode(
                "STEP_LIMIT",
                &format!("{}={}", step.name, step.limit),
            ));
        }
        for policy in &program.cache_policies {
            opcodes.push(opcode(
                "CACHE_BIND",
                &format!("{}<-{}", policy.target, policy.key_source),
            ));
            opcodes.push(opcode(
                "CACHE_TTL_SECONDS",
                &format!("{}={}", policy.target, policy.ttl_seconds),
            ));
        }
        for assertion in &program.assertions {
            opcodes.push(opcode(
                &format!("ASSERT_{}", assertion.predicate.to_ascii_uppercase()),
                &assertion.target,
            ));
        }
        for branch in &program.branches {
            opcodes.push(opcode(
                &format!("IF_{}", branch.predicate.to_ascii_uppercase()),
                &branch.target,
            ));
            opcodes.push(opcode("THEN_EMIT", &branch.then_emit));
            opcodes.push(opcode("ELSE_EMIT", &branch.else_emit));
        }
        if let Some(emit) = &program.emit {
            opcodes.push(opcode("EMIT", emit));
        }
    } else {
        opcodes.extend([
            opcode("GOAL_SHA256", &goal_hash),
            opcode("BUDGET_TOKENS", &program.budget_tokens.to_string()),
            opcode("LIMIT", &program.limit.to_string()),
            opcode("EXEC", &program.mode),
        ]);
    }
    let program_hash = program_hash(&program)?;
    let ast_hash = serde_hash(&ast)?;
    let qir_hash = serde_hash(&qir)?;
    let opcodes_hash = serde_hash(&opcodes)?;
    let qstk_hash = serde_hash(&qstk)?;
    let instruction_count = opcodes.len();

    Ok(QorxBytecode {
        schema: "qorx.bytecode.v1".to_string(),
        language: "qorx".to_string(),
        extension: ".qorxb".to_string(),
        version: program.version.clone(),
        ast,
        qir,
        program,
        opcodes,
        qstk,
        source_tokens,
        goal_hash,
        program_hash,
        ast_hash,
        qir_hash,
        opcodes_hash,
        qstk_hash,
        instruction_count,
        boundary: "Qorx bytecode stores the parsed program in Qorx's protobuf envelope, including qstk, a Forth-inspired stack tape for tiny local dispatch. It is not meaningful to a third-party model without Qorx's resolver/tool contract.".to_string(),
    })
}

fn statement_opcode(op: &str) -> String {
    match op {
        "strict-answer" => "STRICT_ANSWER".to_string(),
        other => other.replace('-', "_").to_ascii_uppercase(),
    }
}

fn qstk_from_program(program: &QorxProgram, goal_hash: &str) -> Vec<QorxStackOp> {
    let mut tape = vec![qword("ver", &program.version), qword("mode", &program.mode)];
    if let Some(handle) = &program.handle {
        tape.push(qword("hndl", handle));
    }

    if program.mode == "program" || program_has_language_items(program) {
        for import in &program.imports {
            match &import.alias {
                Some(alias) => tape.push(qword("usas", &format!("{} {}", import.module, alias))),
                None => tape.push(qword("use", &import.module)),
            }
        }
        for binding in &program.bindings {
            tape.push(qword("lit", &hex_sha256(binding.value.as_bytes())));
            tape.push(qword("bind", &binding.name));
        }
        for step in &program.steps {
            tape.push(qword("src", &step.source));
            tape.push(qword("bud", &step.budget_tokens.to_string()));
            tape.push(qword("lim", &step.limit.to_string()));
            tape.push(qword(
                "call",
                &format!(
                    "{} {}<-{}",
                    statement_opcode(&step.op),
                    step.name,
                    step.source
                ),
            ));
        }
        for policy in &program.cache_policies {
            tape.push(qword("qkey", &policy.key_source));
            tape.push(qword("qttl", &policy.ttl_seconds.to_string()));
            tape.push(qword("qcas", &policy.target));
        }
        for assertion in &program.assertions {
            tape.push(qword(
                "qgat",
                &format!("{}({})", assertion.predicate, assertion.target),
            ));
        }
        for branch in &program.branches {
            tape.push(qword(
                "qif",
                &format!("{}({})", branch.predicate, branch.target),
            ));
            tape.push(qword("then", &branch.then_emit));
            tape.push(qword("qels", &branch.else_emit));
        }
        if let Some(emit) = &program.emit {
            tape.push(qword("emit", emit));
        }
    } else {
        tape.push(qword("goal", goal_hash));
        tape.push(qword("bud", &program.budget_tokens.to_string()));
        tape.push(qword("lim", &program.limit.to_string()));
        tape.push(qword("exec", &program.mode));
    }
    tape
}

fn validate_bytecode(bytecode: &QorxBytecode, path: &Path) -> Result<()> {
    if bytecode.schema != "qorx.bytecode.v1"
        || bytecode.language != "qorx"
        || bytecode.extension != ".qorxb"
    {
        return Err(anyhow!(
            "invalid Qorx bytecode envelope: {}",
            path.display()
        ));
    }
    validate_program(&bytecode.program)?;
    let expected = bytecode_from_program(bytecode.program.clone(), bytecode.source_tokens)?;
    if bytecode.goal_hash != expected.goal_hash {
        return Err(anyhow!(
            "Qorx bytecode goal hash mismatch: {}",
            path.display()
        ));
    }
    if bytecode.program_hash != expected.program_hash {
        return Err(anyhow!(
            "Qorx bytecode program hash mismatch: {}",
            path.display()
        ));
    }
    if bytecode.ast_hash != expected.ast_hash || bytecode.ast != expected.ast {
        return Err(anyhow!("Qorx bytecode AST mismatch: {}", path.display()));
    }
    if bytecode.qir_hash != expected.qir_hash || bytecode.qir != expected.qir {
        return Err(anyhow!("Qorx bytecode QIR mismatch: {}", path.display()));
    }
    if bytecode.opcodes_hash != expected.opcodes_hash || bytecode.opcodes != expected.opcodes {
        return Err(anyhow!(
            "Qorx bytecode opcode stream mismatch: {}",
            path.display()
        ));
    }
    if bytecode.qstk_hash != expected.qstk_hash || bytecode.qstk != expected.qstk {
        return Err(anyhow!(
            "Qorx bytecode qstk stack tape mismatch: {}",
            path.display()
        ));
    }
    if bytecode.instruction_count != expected.instruction_count {
        return Err(anyhow!(
            "Qorx bytecode instruction count mismatch: {}",
            path.display()
        ));
    }
    Ok(())
}

fn require_source_extension(path: &Path) -> Result<()> {
    if normalized_extension(path).as_deref() == Some("qorx") {
        return Ok(());
    }
    Err(anyhow!(
        "expected .qorx Qorx source, got {}",
        path.display()
    ))
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn opcode(op: &str, value: &str) -> QorxOpcode {
    QorxOpcode {
        op: op.to_string(),
        value: value.to_string(),
    }
}

fn qword(word: &str, arg: &str) -> QorxStackOp {
    QorxStackOp {
        word: word.to_string(),
        arg: Some(arg.to_string()),
    }
}

fn program_hash(program: &QorxProgram) -> Result<String> {
    serde_hash(program)
}

fn serde_hash<T: Serialize>(value: &T) -> Result<String> {
    let canonical = serde_json::to_vec(value)?;
    Ok(hex_sha256(&canonical))
}

fn short_hash(hash: &str) -> &str {
    hash.get(..16).unwrap_or(hash)
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn fake_index() -> RepoIndex {
        RepoIndex {
            root: "C:/repo".to_string(),
            updated_at: Utc::now(),
            atoms: vec![index::RepoAtom {
                id: "qva_money".to_string(),
                path: "src/money.rs".to_string(),
                start_line: 1,
                end_line: 3,
                hash: "abc".to_string(),
                token_estimate: 24,
                symbols: vec!["production_gate_passed".to_string()],
                signal_mask: 4,
                vector: vec![1, 2, 3],
                text: "production gate requires routed provider savings evidence".to_string(),
            }],
        }
    }

    #[test]
    fn parses_at_directives_as_qorx_program() {
        let program = parse_program(
            "QORX 1\n@mode agent\n@goal prove production gate provider savings\n@budget 600\n",
        )
        .expect("parse qorx");

        assert_eq!(program.language, "qorx");
        assert_eq!(program.extension, ".qorx");
        assert_eq!(program.mode, "agent");
        assert_eq!(program.goal, "prove production gate provider savings");
        assert_eq!(program.budget_tokens, 600);
    }

    #[test]
    fn runs_strict_answer_without_provider_calls() {
        let program =
            parse_program("mode: strict-answer\nask: production gate routed provider evidence\n")
                .expect("parse qorx");
        let execution = execute_program(&program, &fake_index()).expect("execute qorx");

        assert_eq!(execution["schema"], "qorx.strict-answer.v1");
        assert_eq!(execution["coverage"], "supported");
    }

    #[test]
    fn prompt_contract_tells_models_to_call_qorx() {
        let source = "mode: strict-answer\nask: production gate routed provider evidence\n";
        let tmp = std::env::temp_dir().join(format!(
            "qorx-contract-{}-{}.qorx",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::write(&tmp, source).expect("write source");

        let report = prompt_file(&tmp).expect("prompt report");

        assert_eq!(report.tool.name, "qorx.resolve");
        assert!(report.prompt_block.contains("call qorx.resolve"));
        assert_eq!(report.provider_calls, 0);

        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn bytecode_validation_rejects_tampered_opcode_stream() {
        let program = parse_checked_program(
            r#"QORX 1
let question = "production gate routed provider evidence"
strict answer from question limit 1
emit answer
"#,
        )
        .expect("parse checked program");
        let mut bytecode = bytecode_from_program(program, 32).expect("compile bytecode");

        bytecode.opcodes.push(opcode("EMIT", "forged"));

        let err = validate_bytecode(&bytecode, Path::new("tampered.qorxb"))
            .expect_err("tampered opcode stream must be rejected")
            .to_string();
        assert!(err.contains("opcode stream mismatch"));
    }

    #[test]
    fn bytecode_validation_rejects_tampered_qstk_stream() {
        let program = parse_checked_program(
            r#"QORX 1
let question = "production gate routed provider evidence"
strict answer from question limit 1
emit answer
"#,
        )
        .expect("parse checked program");
        let mut bytecode = bytecode_from_program(program, 32).expect("compile bytecode");

        bytecode.qstk.push(qword("emit", "forged"));

        let err = validate_bytecode(&bytecode, Path::new("tampered-qstk.qorxb"))
            .expect_err("tampered qstk stack tape must be rejected")
            .to_string();
        assert!(err.contains("qstk stack tape mismatch"));
    }
}
