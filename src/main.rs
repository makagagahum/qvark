#![allow(dead_code)]

mod a2a;
mod adapters;
mod aim;
mod b2c_quant;
mod cache_plan;
mod capsule;
mod compression;
mod config;
mod context_proto;
mod context_vm;
mod cosmos;
mod cost_stack;
mod impact;
mod index;
mod judge;
mod kv;
mod lattice;
mod lexicon;
mod memory;
mod money;
mod proto_store;
mod qorx;
mod response_cache;
mod security;
mod session;
mod share;
mod squeeze;
mod stats;
mod text;
mod truth;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::{fs, path::PathBuf};

use crate::config::AppPaths;

const DEFAULT_PRO_DRIVE_SIZE: &str = "2G";

#[derive(Debug, Parser)]
#[command(
    name = "qorx",
    version,
    about = "Qorx language and runtime for local context resolution"
)]
struct Args {
    #[command(subcommand)]
    command: Option<CommandKind>,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    Bootstrap {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        no_integrations: bool,
    },
    Daemon {
        #[command(subcommand)]
        action: Option<DaemonAction>,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
    Tray,
    Stats {
        #[command(subcommand)]
        action: Option<StatsAction>,
    },
    Money {
        #[arg(long = "claim-usd")]
        claim_usd: Option<f64>,
    },
    Search {
        query: String,
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
    },
    StrictAnswer {
        question: String,
        #[arg(short, long, default_value_t = 2)]
        limit: usize,
    },
    Squeeze {
        query: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
    },
    Judge {
        answer: String,
        #[arg(short, long)]
        query: Option<String>,
    },
    CachePlan {
        prompt: String,
    },
    #[command(name = "b2c-plan")]
    B2cPlan {
        query: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
    },
    Agent {
        objective: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
    },
    Marvin {
        objective: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
    },
    Pack {
        query: String,
        #[arg(short = 'b', long, default_value_t = 4_000)]
        budget_tokens: u64,
    },
    Impact {
        query: String,
        #[arg(short = 'b', long, default_value_t = 4_000)]
        budget_tokens: u64,
        #[arg(long)]
        diff: Option<String>,
        #[arg(long = "diff-file")]
        diff_file: Option<PathBuf>,
    },
    Map {
        query: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(long)]
        diff: Option<String>,
        #[arg(long = "diff-file")]
        diff_file: Option<PathBuf>,
    },
    Qorx {
        file: PathBuf,
    },
    QorxCompile {
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    QorxCheck {
        input: PathBuf,
    },
    QorxInspect {
        file: PathBuf,
    },
    QorxPrompt {
        file: PathBuf,
        #[arg(long)]
        block: bool,
    },
    A2a {
        #[command(subcommand)]
        action: A2aAction,
    },
    Cosmos {
        #[command(subcommand)]
        action: CosmosAction,
    },
    Lexicon,
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    Lattice {
        #[command(subcommand)]
        action: LatticeAction,
    },
    Share {
        #[command(subcommand)]
        action: ShareAction,
    },
    Kv {
        #[command(subcommand)]
        action: KvAction,
    },
    Attest {
        #[arg(long)]
        formal: bool,
        #[arg(long, default_value_t = 3)]
        level: u8,
    },
    Bench {
        #[arg(short = 'b', long, default_value_t = 4_000)]
        budget_tokens: u64,
        queries: Vec<String>,
    },
    Adapters,
    Science,
    Aim,
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },
    Hot {
        #[command(subcommand)]
        action: HotAction,
    },
    Capsule {
        #[command(subcommand)]
        action: CapsuleAction,
    },
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },
    Session {
        #[arg(long)]
        block: bool,
    },
    Startup {
        #[command(subcommand)]
        action: StartupAction,
    },
    Portable {
        #[command(subcommand)]
        action: PortableAction,
    },
    Drive {
        #[command(subcommand)]
        action: DriveAction,
    },
    Integrate {
        #[command(subcommand)]
        action: IntegrateAction,
    },
    Index {
        path: PathBuf,
    },
    Run {
        provider: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Patch {
        provider: String,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum StartupAction {
    Enable,
    Disable,
    Status,
}

#[derive(Debug, Clone, Subcommand)]
enum DaemonAction {
    Run,
    Start,
    Stop,
    Status,
}

#[derive(Debug, Clone, Subcommand)]
enum StatsAction {
    Reset,
}

#[derive(Debug, Clone, Subcommand)]
enum A2aAction {
    Card,
    Task { file: PathBuf },
}

#[derive(Debug, Clone, Subcommand)]
enum CosmosAction {
    Status,
}

#[derive(Debug, Clone, Subcommand)]
enum MemoryAction {
    Create {
        kind: String,
        text: String,
    },
    Read {
        query: String,
        #[arg(short, long, default_value_t = 8)]
        limit: usize,
    },
    Update {
        id: String,
        text: String,
    },
    Delete {
        id: String,
    },
    Summarize {
        #[arg(short, long, default_value_t = 8)]
        limit: usize,
    },
    Prune {
        #[arg(long = "max-items", default_value_t = 64)]
        max_items: usize,
    },
    Gc {
        #[arg(long, default_value = "lattice")]
        strategy: String,
        #[arg(long = "max-items", default_value_t = 64)]
        max_items: usize,
    },
    Evolve {
        #[arg(long)]
        task: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum LatticeAction {
    Build {
        #[arg(long)]
        task: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
    },
    Status,
    Attest {
        #[arg(long)]
        formal: bool,
    },
    KvHints {
        #[arg(long)]
        task: Option<String>,
    },
    EvolveRules {
        #[arg(long)]
        task: String,
    },
    Rules,
}

#[derive(Debug, Clone, Subcommand)]
enum ShareAction {
    Export {
        #[arg(long)]
        out: PathBuf,
    },
    Capsule {
        #[arg(long)]
        capsule: Option<String>,
        #[arg(long)]
        to: PathBuf,
    },
    Import {
        bundle: PathBuf,
    },
    Session {
        #[arg(long)]
        block: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum KvAction {
    Emit {
        #[arg(long)]
        model: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum PortableAction {
    Init,
    Status,
}

#[derive(Debug, Clone, Subcommand)]
enum DriveAction {
    Init {
        #[arg(short, long, default_value = "Q")]
        letter: String,
        #[arg(long)]
        ram: bool,
        #[arg(long, default_value = DEFAULT_PRO_DRIVE_SIZE)]
        size: String,
    },
    Mount {
        #[arg(short, long, default_value = "Q")]
        letter: String,
        #[arg(long)]
        ram: bool,
        #[arg(long, default_value = DEFAULT_PRO_DRIVE_SIZE)]
        size: String,
    },
    Unmount {
        #[arg(short, long, default_value = "Q")]
        letter: String,
    },
    Status {
        #[arg(short, long, default_value = "Q")]
        letter: String,
        #[arg(long)]
        ram: bool,
    },
    InstallStartup {
        #[arg(short, long, default_value = "Q")]
        letter: String,
        #[arg(long)]
        ram: bool,
        #[arg(long, default_value = DEFAULT_PRO_DRIVE_SIZE)]
        size: String,
    },
    RemoveStartup {
        #[arg(short, long, default_value = "Q")]
        letter: String,
    },
    InstallImdisk {
        bundle: PathBuf,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum SecurityAction {
    Attest,
    Verify,
}

#[derive(Debug, Clone, Subcommand)]
enum HotAction {
    Status,
    Install {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(short, long, default_value = "Q")]
        letter: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum ContextAction {
    Snapshot,
    Verify,
    Vm {
        objective: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
        #[arg(long)]
        block: bool,
    },
    Fault {
        query: String,
        #[arg(long)]
        handle: Option<String>,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
    },
    Inject {
        objective: Option<String>,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
        #[arg(long)]
        block: bool,
    },
    Nano {
        objective: Option<String>,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
        #[arg(long)]
        block: bool,
    },
    Quetta {
        objective: Option<String>,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
        #[arg(long)]
        block: bool,
    },
    Expand {
        carrier: String,
        #[arg(short = 'b', long, default_value_t = 900)]
        budget_tokens: u64,
        #[arg(short, long, default_value_t = 4)]
        limit: usize,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum CapsuleAction {
    Auto {
        #[arg(long)]
        block: bool,
        #[arg(long = "max-files")]
        max_files: Option<usize>,
    },
    Detect,
    Create {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        include_memory: bool,
        #[arg(long)]
        include_aim: bool,
        #[arg(long)]
        include_sensitive: bool,
        #[arg(long = "max-files")]
        max_files: Option<usize>,
        #[arg(long)]
        block: bool,
    },
    Session {
        #[arg(long)]
        block: bool,
    },
    StrictAnswer {
        question: String,
        #[arg(short, long, default_value_t = 2)]
        limit: usize,
    },
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    schema: String,
    version: String,
    tier: String,
    shared_service_ready: bool,
    gateway_healthy: bool,
    bind: String,
    data_dir: String,
    index_present: bool,
    stats_present: bool,
    response_cache_present: bool,
    provenance_present: bool,
    package_surfaces: Vec<String>,
    production_checks: Vec<String>,
    shared_service_gaps: Vec<String>,
    boundary: String,
}

impl DoctorReport {
    async fn collect() -> Result<Self> {
        let paths = AppPaths::resolve()?;
        Ok(Self {
            schema: "qorx.doctor.v1".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            tier: "community source edition".to_string(),
            shared_service_ready: false,
            gateway_healthy: false,
            bind: "not exposed in Community Edition".to_string(),
            data_dir: paths.data_dir.display().to_string(),
            index_present: paths.index_file.exists(),
            stats_present: paths.stats_file.exists(),
            response_cache_present: paths.response_cache_file.exists(),
            provenance_present: paths.provenance_file.exists(),
            package_surfaces: vec![
                "cargo install --git".to_string(),
                "source build with Cargo".to_string(),
            ],
            production_checks: vec![
                "qorx --version".to_string(),
                "qorx index <repo>".to_string(),
                "qorx security attest".to_string(),
            ],
            shared_service_gaps: vec![
                "No built-in multi-user authentication or authorization layer".to_string(),
                "No tenant isolation model".to_string(),
                "No published external load-test SLO".to_string(),
                "No managed upgrade or migration controller".to_string(),
            ],
            boundary: "This public repository is the AGPL Community Edition. Official binaries, tray, auto-start, provider proxy routing, one-click integrations, updater, hosted account features, and managed local-vault UX are reserved for Qorx Local Pro.".to_string(),
        })
    }

    fn print_human(&self) {
        println!("Qorx {}", self.version);
        println!("tier: {}", self.tier);
        println!("gateway_healthy: {}", self.gateway_healthy);
        println!("bind: {}", self.bind);
        println!("data_dir: {}", self.data_dir);
        println!("index_present: {}", self.index_present);
        println!("stats_present: {}", self.stats_present);
        println!("response_cache_present: {}", self.response_cache_present);
        println!("provenance_present: {}", self.provenance_present);
        println!("shared_service_ready: {}", self.shared_service_ready);
        println!("boundary: {}", self.boundary);
    }
}

fn pro_only(feature: &str) -> Result<()> {
    anyhow::bail!(
        "{feature} is not included in Qorx Community Edition. Build the source CLI for language, indexing, and evidence commands. Official background runtime, tray, provider routing, startup, drive, and integration activation are Qorx Local Pro surfaces."
    )
}

fn print_stats(paths: &AppPaths) -> Result<()> {
    let legacy = paths.stats_file.with_extension("json");
    let stats: stats::Stats = proto_store::load_or_default(&paths.stats_file, &[legacy.as_path()])?;
    println!("{}", serde_json::to_string(&stats)?);
    Ok(())
}

fn reset_stats(paths: &AppPaths) -> Result<()> {
    let stats = stats::reset(&paths.stats_file)?;
    println!("{}", serde_json::to_string(&stats)?);
    Ok(())
}

#[derive(Debug, Clone, Subcommand)]
enum IntegrateAction {
    Activate,
    Deactivate,
    Status,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let args = Args::parse();
    let command = args.command.unwrap_or(CommandKind::Doctor { json: false });
    match command {
        CommandKind::Bootstrap {
            json: _,
            path: _,
            no_integrations: _,
        } => {
            pro_only("bootstrap activation")?;
        }
        CommandKind::Daemon { action } => match action.unwrap_or(DaemonAction::Run) {
            DaemonAction::Run => {
                pro_only("local HTTP daemon")?;
            }
            DaemonAction::Start => {
                pro_only("daemon start")?;
            }
            DaemonAction::Stop => {
                pro_only("daemon stop")?;
            }
            DaemonAction::Status => {
                pro_only("daemon status")?;
            }
        },
        CommandKind::Doctor { json } => {
            let report = DoctorReport::collect().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                report.print_human();
            }
        }
        CommandKind::Tray => {
            pro_only("Windows tray")?;
        }
        CommandKind::Stats { action } => {
            let paths = AppPaths::resolve()?;
            match action {
                Some(StatsAction::Reset) => reset_stats(&paths)?,
                None => print_stats(&paths)?,
            }
        }
        CommandKind::Money { claim_usd } => {
            let paths = AppPaths::resolve()?;
            let legacy = paths.stats_file.with_extension("json");
            let stats: stats::Stats =
                proto_store::load_or_default(&paths.stats_file, &[legacy.as_path()])?;
            let proof = money::build_money_proof(&stats, claim_usd);
            println!("{}", serde_json::to_string_pretty(&proof)?);
        }
        CommandKind::Search { query, limit } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let hits = index::search_index(&index, &query, limit);
            println!("{}", serde_json::to_string_pretty(&hits)?);
        }
        CommandKind::StrictAnswer { question, limit } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let answer = truth::strict_answer(&index, &question, limit);
            println!("{}", serde_json::to_string_pretty(&answer)?);
        }
        CommandKind::Squeeze {
            query,
            budget_tokens,
            limit,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let report = squeeze::squeeze_context(&index, &query, budget_tokens, limit);
            let _ = stats::record_context_pack(
                &paths.stats_file,
                report.indexed_tokens,
                report.used_tokens,
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Judge { answer, query } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let report = judge::judge_answer(&index, &answer, query.as_deref());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::CachePlan { prompt } => {
            let report = cache_plan::plan_prompt(&prompt);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::B2cPlan {
            query,
            budget_tokens,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let report = b2c_quant::plan_context(&index, &query, budget_tokens);
            let _ = stats::record_context_pack(
                &paths.stats_file,
                report.indexed_tokens,
                report.used_tokens,
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Agent {
            objective,
            budget_tokens,
        }
        | CommandKind::Marvin {
            objective,
            budget_tokens,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let report = truth::run_agent(&index, &objective, budget_tokens);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Pack {
            query,
            budget_tokens,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let packed = index::pack_context(&index, &query, budget_tokens);
            let _ = stats::record_context_pack(
                &paths.stats_file,
                packed.indexed_tokens,
                packed.used_tokens,
            );
            println!("{}", serde_json::to_string_pretty(&packed)?);
        }
        CommandKind::Impact {
            query,
            budget_tokens,
            diff,
            diff_file,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let diff_text = match diff_file {
                Some(path) => Some(fs::read_to_string(path)?),
                None => diff,
            };
            let packed =
                impact::impact_context(&index, &query, diff_text.as_deref(), budget_tokens);
            let _ = stats::record_context_pack(
                &paths.stats_file,
                packed.indexed_tokens,
                packed.used_tokens,
            );
            println!("{}", serde_json::to_string_pretty(&packed)?);
        }
        CommandKind::Map {
            query,
            budget_tokens,
            diff,
            diff_file,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let diff_text = match diff_file {
                Some(path) => Some(fs::read_to_string(path)?),
                None => diff,
            };
            let mapped = impact::map_context(&index, &query, diff_text.as_deref(), budget_tokens);
            let _ = stats::record_context_pack(
                &paths.stats_file,
                mapped.indexed_tokens,
                mapped.used_tokens,
            );
            println!("{}", serde_json::to_string_pretty(&mapped)?);
        }
        CommandKind::Qorx { file } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let report = qorx::run_file(&file, &index)?;
            let cosmos = cosmos::record_run(&paths, "qorx.run", &report)?;
            let mut value = serde_json::to_value(&report)?;
            if let serde_json::Value::Object(map) = &mut value {
                map.insert(
                    "lexicon".to_string(),
                    lexicon::runtime_tags(&report.source_kind),
                );
                map.insert("cosmos".to_string(), serde_json::to_value(cosmos)?);
            }
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        CommandKind::QorxCompile { input, out } => {
            let report = qorx::compile_file(&input, out.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::QorxCheck { input } => {
            let report = qorx::check_file(&input)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::QorxInspect { file } => {
            let report = qorx::inspect_file(&file)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::QorxPrompt { file, block } => {
            let report = qorx::prompt_file(&file)?;
            if block {
                println!("{}", report.prompt_block);
            } else {
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
        }
        CommandKind::A2a { action } => match action {
            A2aAction::Card => {
                println!("{}", serde_json::to_string_pretty(&a2a::agent_card())?);
            }
            A2aAction::Task { file } => {
                let paths = AppPaths::resolve()?;
                let index = index::load_index(&paths.index_file)?;
                let response = a2a::task_from_file(&file, &index, Some(&paths))?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
        },
        CommandKind::Cosmos { action } => match action {
            CosmosAction::Status => {
                let paths = AppPaths::resolve()?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cosmos::status(&paths)?)?
                );
            }
        },
        CommandKind::Lexicon => {
            println!("{}", serde_json::to_string_pretty(&lexicon::report())?);
        }
        CommandKind::Memory { action } => {
            let paths = AppPaths::resolve()?;
            let report = match action {
                MemoryAction::Create { kind, text } => {
                    serde_json::to_value(memory::create(&paths, &kind, &text)?)?
                }
                MemoryAction::Read { query, limit } => {
                    serde_json::to_value(memory::read(&paths, &query, limit)?)?
                }
                MemoryAction::Update { id, text } => {
                    serde_json::to_value(memory::update(&paths, &id, &text)?)?
                }
                MemoryAction::Delete { id } => serde_json::to_value(memory::delete(&paths, &id)?)?,
                MemoryAction::Summarize { limit } => {
                    serde_json::to_value(memory::summarize(&paths, limit)?)?
                }
                MemoryAction::Prune { max_items } => {
                    serde_json::to_value(memory::prune(&paths, max_items)?)?
                }
                MemoryAction::Gc {
                    strategy,
                    max_items,
                } => serde_json::to_value(memory::gc(&paths, &strategy, max_items)?)?,
                MemoryAction::Evolve {
                    task,
                    budget_tokens,
                } => serde_json::to_value(lattice::evolve(&paths, &task, budget_tokens)?)?,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Lattice { action } => {
            let paths = AppPaths::resolve()?;
            let report = match action {
                LatticeAction::Build {
                    task,
                    budget_tokens,
                } => {
                    let lattice = lattice::build(&paths, &task, budget_tokens)?;
                    proto_store::save(&lattice::lattice_path(&paths), &lattice)?;
                    serde_json::to_value(lattice)?
                }
                LatticeAction::Status => match lattice::status(&paths) {
                    Ok(report) => serde_json::to_value(report)?,
                    Err(_) => return Err(lattice::missing_lattice_error()),
                },
                LatticeAction::Attest { formal } => {
                    serde_json::to_value(lattice::attest(&paths, formal)?)?
                }
                LatticeAction::KvHints { task } => {
                    serde_json::to_value(lattice::kv_hint_export(&paths, task.as_deref())?)?
                }
                LatticeAction::EvolveRules { task } => {
                    serde_json::to_value(lattice::evolve_rules(&paths, &task)?)?
                }
                LatticeAction::Rules => serde_json::to_value(lattice::load_rules(&paths)?)?,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Share { action } => {
            let paths = AppPaths::resolve()?;
            match action {
                ShareAction::Export { out } => {
                    let report = share::export(&paths, &out)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                ShareAction::Capsule { capsule, to } => {
                    let report = share::export_capsule(&paths, capsule.as_deref(), &to)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                ShareAction::Import { bundle } => {
                    let report = share::import(&paths, &bundle)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                ShareAction::Session { block } => {
                    let report = share::session(&paths)?;
                    if block {
                        println!("{}", report.prompt_block);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
            }
        }
        CommandKind::Kv { action } => {
            let paths = AppPaths::resolve()?;
            match action {
                KvAction::Emit { model, task, out } => {
                    let report = kv::emit(&paths, &model, task.as_deref(), out)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
        }
        CommandKind::Attest { formal, level } => {
            let paths = AppPaths::resolve()?;
            let report = lattice::formal_attest(&paths, formal, level)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Bench {
            budget_tokens,
            queries,
        } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let queries = if queries.is_empty() {
                vec![
                    "provider cached tokens prompt cache".to_string(),
                    "repo quark pack context benchmark".to_string(),
                    "kv cache rotorquant adapter".to_string(),
                ]
            } else {
                queries
            };
            let report = index::benchmark_queries(&index, &queries, budget_tokens);
            for row in &report.rows {
                let _ = stats::record_context_pack(
                    &paths.stats_file,
                    report.indexed_tokens,
                    row.used_tokens,
                );
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Adapters => {
            println!(
                "{}",
                serde_json::to_string_pretty(&adapters::adapter_report())?
            );
        }
        CommandKind::Science => {
            println!(
                "{}",
                serde_json::to_string_pretty(&adapters::science_report())?
            );
        }
        CommandKind::Aim => {
            println!(
                "{}",
                serde_json::to_string_pretty(&aim::inspect_default()?)?
            );
        }
        CommandKind::Security { action } => {
            let paths = AppPaths::resolve()?;
            let report = match action {
                SecurityAction::Attest => security::attest(&paths)?,
                SecurityAction::Verify => security::verify_saved(&paths)?,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Hot { action } => {
            let _ = action;
            pro_only("hot local vault install")?;
        }
        CommandKind::Context { action } => {
            let paths = AppPaths::resolve()?;
            match action {
                ContextAction::Snapshot => {
                    let report = context_proto::snapshot(&paths)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                ContextAction::Verify => {
                    let report = context_proto::verify(&paths)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                ContextAction::Vm {
                    objective,
                    budget_tokens,
                    limit,
                    block,
                } => {
                    let index = index::load_index(&paths.index_file)?;
                    let report = context_vm::build_context_vm(
                        &index,
                        &objective,
                        context_vm::ContextVmOptions {
                            budget_tokens,
                            limit,
                        },
                    );
                    let _ = stats::record_context_pack(
                        &paths.stats_file,
                        report.ledger.indexed_tokens,
                        report.ledger.sent_tokens,
                    );
                    if block {
                        println!("{}", report.prompt_block);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                ContextAction::Fault {
                    query,
                    handle,
                    budget_tokens,
                    limit,
                } => {
                    let index = index::load_index(&paths.index_file)?;
                    let session = session::build_session_pointer(&index);
                    let handle = handle.unwrap_or(session.handle);
                    let report = context_vm::resolve_context_fault(
                        &index,
                        &handle,
                        &query,
                        context_vm::ContextVmOptions {
                            budget_tokens,
                            limit,
                        },
                    );
                    let _ = stats::record_context_pack(
                        &paths.stats_file,
                        report.indexed_tokens,
                        report.used_tokens,
                    );
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                ContextAction::Inject {
                    objective,
                    budget_tokens,
                    limit,
                    block,
                } => {
                    let index = index::load_index(&paths.index_file)?;
                    let objective = objective.unwrap_or_else(|| "current agent turn".to_string());
                    let report = context_vm::build_context_injection(
                        &index,
                        &objective,
                        context_vm::ContextVmOptions {
                            budget_tokens,
                            limit,
                        },
                    );
                    if block {
                        println!("{}", report.additional_context);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                ContextAction::Nano {
                    objective,
                    budget_tokens,
                    limit,
                    block,
                } => {
                    let index = index::load_index(&paths.index_file)?;
                    let objective = objective.unwrap_or_else(|| "current agent turn".to_string());
                    let report = context_vm::build_context_nano(
                        &index,
                        &objective,
                        context_vm::ContextVmOptions {
                            budget_tokens,
                            limit,
                        },
                    );
                    let _ = stats::record_context_pack(
                        &paths.stats_file,
                        report.indexed_tokens,
                        report.visible_tokens,
                    );
                    if block {
                        println!("{}", report.carrier);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                ContextAction::Quetta {
                    objective,
                    budget_tokens,
                    limit,
                    block,
                } => {
                    let index = index::load_index(&paths.index_file)?;
                    let objective = objective.unwrap_or_else(|| "current agent turn".to_string());
                    let report = context_vm::build_context_quetta(
                        &index,
                        &objective,
                        context_vm::ContextVmOptions {
                            budget_tokens,
                            limit,
                        },
                    );
                    let _ = stats::record_context_pack(
                        &paths.stats_file,
                        report.local_indexed_tokens,
                        report.visible_tokens,
                    );
                    if block {
                        println!("{}", report.carrier);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                ContextAction::Expand {
                    carrier,
                    budget_tokens,
                    limit,
                } => {
                    let index = index::load_index(&paths.index_file)?;
                    let report = context_vm::expand_nano_carrier(
                        &index,
                        &carrier,
                        context_vm::ContextVmOptions {
                            budget_tokens,
                            limit,
                        },
                    );
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
        }
        CommandKind::Capsule { action } => {
            let paths = AppPaths::resolve()?;
            match action {
                CapsuleAction::Auto { block, max_files } => {
                    let report = capsule::create_auto(
                        &paths,
                        capsule::CapsuleCreateOptions {
                            include_memory: true,
                            include_aim: true,
                            include_sensitive: false,
                            max_files,
                        },
                    )?;
                    if block {
                        println!("{}", report.capsule.prompt_block);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                CapsuleAction::Detect => {
                    let candidates = capsule::detect_brvin_candidates();
                    println!("{}", serde_json::to_string_pretty(&candidates)?);
                }
                CapsuleAction::Create {
                    path,
                    include_memory,
                    include_aim,
                    include_sensitive,
                    max_files,
                    block,
                } => {
                    let report = capsule::create(
                        &paths,
                        &path,
                        capsule::CapsuleCreateOptions {
                            include_memory,
                            include_aim,
                            include_sensitive,
                            max_files,
                        },
                    )?;
                    if block {
                        println!("{}", report.prompt_block);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                CapsuleAction::Session { block } => {
                    if block {
                        let report = capsule::load_session_pointer(&paths)?;
                        println!("{}", report.prompt_block);
                    } else {
                        let report = capsule::load(&paths)?;
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
                CapsuleAction::StrictAnswer { question, limit } => {
                    let report = capsule::strict_answer(&paths, &question, limit)?;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
        }
        CommandKind::Session { block } => {
            let paths = AppPaths::resolve()?;
            let index = index::load_index(&paths.index_file)?;
            let pointer = session::build_session_pointer(&index);
            if block {
                println!("{}", pointer.prompt_block);
            } else {
                println!("{}", serde_json::to_string_pretty(&pointer)?);
            }
        }
        CommandKind::Startup { action } => {
            let _ = action;
            pro_only("daemon startup integration")?;
        }
        CommandKind::Portable { action } => {
            let report = match action {
                PortableAction::Init => config::init_portable()?,
                PortableAction::Status => {
                    let paths = AppPaths::resolve()?;
                    config::portable_report(&paths)?
                }
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CommandKind::Drive { action } => {
            let _ = action;
            pro_only("local drive/vault mounting")?;
        }
        CommandKind::Integrate { action } => {
            let _ = action;
            pro_only("CLI integration activation")?;
        }
        CommandKind::Index { path } => {
            let paths = AppPaths::resolve()?;
            let index = index::build_index(&path, &paths.index_file)?;
            let symbol_count: usize = index.atoms.iter().map(|atom| atom.symbols.len()).sum();
            let signal_count: u32 = index
                .atoms
                .iter()
                .map(|atom| atom.signal_mask.count_ones())
                .sum();
            println!(
                "Indexed {} quarks from {} into {} ({} estimated tokens, {} symbols, {} signals, {} sparse vector terms)",
                index.atoms.len(),
                index.root,
                paths.index_file.display(),
                index.total_tokens(),
                symbol_count,
                signal_count,
                index.vector_terms()
            );
        }
        CommandKind::Run { provider, args } => {
            let _ = (provider, args);
            pro_only("provider routing")?;
        }
        CommandKind::Patch { provider, apply } => {
            let _ = (provider, apply);
            pro_only("provider patching")?;
        }
    }
    Ok(())
}
