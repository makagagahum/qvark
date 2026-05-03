use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    compression::estimate_tokens,
    index::{pack_context, search_index, PackedContext, RepoAtom, RepoIndex},
    session::{build_session_pointer, SessionPointer},
    text::without_string_literals,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrictAnswer {
    pub schema: String,
    pub question: String,
    pub coverage: String,
    pub answer: String,
    pub evidence: Vec<StrictEvidence>,
    pub supported_terms: Vec<String>,
    pub missing_terms: Vec<String>,
    pub indexed_tokens: u64,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrictEvidence {
    pub id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub score: u64,
    pub matched_terms: Vec<String>,
    pub excerpt_hash: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub action: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContract {
    pub hallucination_policy: String,
    pub error_policy: String,
    pub compression_policy: String,
    pub b2c_policy: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReport {
    pub schema: String,
    pub agent_name: String,
    pub objective: String,
    pub mode: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub contract: AgentContract,
    pub steps: Vec<AgentStep>,
    pub session: SessionPointer,
    pub strict_answer: StrictAnswer,
    pub packed_context: PackedContext,
    pub boundary: String,
}

pub fn strict_answer(index: &RepoIndex, question: &str, limit: usize) -> StrictAnswer {
    let limit = limit.clamp(1, 8);
    let indexed_tokens = index.total_tokens();
    let query_terms = meaningful_terms(question);
    let mut supported_terms = BTreeSet::new();
    let mut evidence = Vec::new();
    let atoms_by_id = index.atom_lookup();

    for hit in search_index(index, question, 64) {
        let Some(atom) = atoms_by_id.get(hit.id.as_str()).copied() else {
            continue;
        };
        let matched_terms = matched_terms(atom, &query_terms);
        if !qualifies(&query_terms, &matched_terms) {
            continue;
        }
        let excerpt = select_excerpt(atom, &query_terms);
        if is_fixture_echo(atom, &excerpt) {
            continue;
        }
        for term in &matched_terms {
            supported_terms.insert(term.clone());
        }
        evidence.push(StrictEvidence {
            id: atom.id.clone(),
            path: atom.path.clone(),
            start_line: atom.start_line,
            end_line: atom.end_line,
            score: hit.score,
            matched_terms,
            excerpt_hash: excerpt_hash(&excerpt),
            excerpt,
        });
        if evidence.len() >= limit {
            break;
        }
    }

    let supported_terms = supported_terms.into_iter().collect::<Vec<_>>();
    let missing_terms = query_terms
        .iter()
        .filter(|term| !supported_terms.contains(term))
        .cloned()
        .collect::<Vec<_>>();
    let should_refuse_sensitive_partial = missing_terms
        .iter()
        .any(|term| is_sensitive_refusal_term(term));
    if should_refuse_sensitive_partial {
        evidence.clear();
    }
    let coverage = if evidence.is_empty() {
        "not_found"
    } else if missing_terms.is_empty() {
        "supported"
    } else {
        "partial"
    }
    .to_string();
    let answer = if evidence.is_empty() {
        String::new()
    } else {
        evidence
            .iter()
            .map(|item| item.excerpt.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let used_tokens = estimate_tokens(question)
        + evidence
            .iter()
            .map(|item| estimate_tokens(&item.excerpt) + 18)
            .sum::<u64>();
    let omitted_tokens = indexed_tokens.saturating_sub(used_tokens.min(indexed_tokens));

    StrictAnswer {
        schema: "qorx.strict-answer.v1".to_string(),
        question: question.to_string(),
        coverage,
        answer,
        evidence,
        supported_terms,
        missing_terms,
        indexed_tokens,
        used_tokens,
        omitted_tokens,
        boundary: "Strict answers are extractive: every answer byte comes from indexed quark excerpts. Unsupported questions return not_found instead of model guesses.".to_string(),
    }
}

pub fn run_agent(index: &RepoIndex, objective: &str, budget_tokens: u64) -> AgentReport {
    let budget_tokens = budget_tokens.clamp(128, 20_000);
    let session = build_session_pointer(index);
    let strict = strict_answer(index, objective, 2);
    let packed = pack_context(index, objective, budget_tokens);
    let steps = vec![
        AgentStep {
            action: "session".to_string(),
            status: "completed".to_string(),
            reason: "emit tiny local session pointer without bulk context".to_string(),
        },
        AgentStep {
            action: "strict-answer".to_string(),
            status: strict.coverage.clone(),
            reason: "ground objective against indexed quarks only".to_string(),
        },
        AgentStep {
            action: "pack".to_string(),
            status: "completed".to_string(),
            reason: "return the smallest relevant working set under budget".to_string(),
        },
    ];

    AgentReport {
        schema: "qorx.agent.v1".to_string(),
        agent_name: "Marvin".to_string(),
        objective: objective.to_string(),
        mode: "deterministic_subatomic".to_string(),
        local_only: true,
        provider_calls: 0,
        contract: AgentContract {
            hallucination_policy: "refuse_unsupported_indexed_context".to_string(),
            error_policy: "strict_extract_then_refuse".to_string(),
            compression_policy: "subatomic_context_budget".to_string(),
            b2c_policy: "account_then_claim".to_string(),
            boundary: "Marvin enforces strict indexed-context mode for local evidence and keeps the working set subatomic: small enough for the requested token budget. Subatomic is a Qorx context-size label, not a physics claim. Downstream model correctness, external tool behavior, and facts absent from the local index are outside this local planner contract.".to_string(),
        },
        steps,
        session,
        strict_answer: strict,
        packed_context: packed,
        boundary: "Marvin is Qorx's deterministic local planner. The planner uses local indexed evidence, zero provider calls, and explicit refusal for unsupported indexed context.".to_string(),
    }
}

fn meaningful_terms(text: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
            continue;
        }
        push_term(&mut terms, &mut current);
    }
    push_term(&mut terms, &mut current);
    terms.into_iter().collect()
}

fn push_term(terms: &mut BTreeSet<String>, current: &mut String) {
    if current.len() < 3 {
        current.clear();
        return;
    }
    if is_stopword(current) {
        current.clear();
        return;
    }
    terms.insert(std::mem::take(current));
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "about"
            | "all"
            | "and"
            | "are"
            | "can"
            | "does"
            | "for"
            | "from"
            | "how"
            | "into"
            | "our"
            | "prove"
            | "qorx"
            | "show"
            | "tell"
            | "that"
            | "the"
            | "this"
            | "what"
            | "when"
            | "where"
            | "why"
            | "with"
    )
}

fn is_sensitive_refusal_term(term: &str) -> bool {
    matches!(
        term,
        "admin"
            | "administrator"
            | "api"
            | "credential"
            | "credentials"
            | "key"
            | "password"
            | "secret"
            | "secrets"
            | "token"
            | "tokens"
    )
}

fn matched_terms(atom: &RepoAtom, query_terms: &[String]) -> Vec<String> {
    let haystack = searchable_text(atom);
    query_terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .cloned()
        .collect()
}

fn searchable_text(atom: &RepoAtom) -> String {
    let body = if is_code_path(&atom.path) {
        without_string_literals(&atom.text)
    } else {
        atom.text.clone()
    };
    format!("{} {} {}", atom.path, atom.symbols.join(" "), body).to_lowercase()
}

fn is_code_path(path: &str) -> bool {
    matches!(
        path.rsplit('.').next().unwrap_or_default(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "py"
            | "go"
            | "java"
            | "kt"
            | "swift"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
            | "php"
            | "rb"
    )
}

fn qualifies(query_terms: &[String], matched_terms: &[String]) -> bool {
    if query_terms.is_empty() {
        return false;
    }
    let minimum = if query_terms.len() <= 2 { 1 } else { 2 };
    matched_terms.len() >= minimum
}

fn select_excerpt(atom: &RepoAtom, query_terms: &[String]) -> String {
    let mut ranked = atom
        .text
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let lower = trimmed.to_lowercase();
            let score = query_terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count();
            if score == 0 {
                return None;
            }
            Some((idx, score, trimmed.to_string()))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(4);
    ranked.sort_by_key(|a| a.0);

    let selected = if ranked.is_empty() {
        atom.text.trim().to_string()
    } else {
        ranked
            .into_iter()
            .map(|(_, _, line)| line)
            .collect::<Vec<_>>()
            .join("\n")
    };
    truncate_chars(&selected, 600)
}

fn is_fixture_echo(atom: &RepoAtom, excerpt: &str) -> bool {
    let path = atom.path.replace('\\', "/").to_lowercase();
    if !path.starts_with("tests/") && !path.contains("/tests/") {
        return false;
    }
    let text = format!("{}\n{}", atom.text, excerpt).to_lowercase();
    text.contains(".args(") && (text.contains("strict-answer") || text.contains("agent"))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

fn excerpt_hash(excerpt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(excerpt.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::index::{RepoAtom, RepoIndex};

    #[test]
    fn strict_answer_ignores_cli_fixture_echoes_in_test_sources() {
        let index = RepoIndex {
            root: "C:/repo".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_fixture".to_string(),
                path: "tests/truth_kernel.rs".to_string(),
                start_line: 1,
                end_line: 8,
                hash: "fixture".to_string(),
                token_estimate: 20,
                symbols: vec!["strict_answer_fixture".to_string()],
                signal_mask: 0,
                vector: vec![],
                text: r##"let fixture = r#"
Command::new(env!("CARGO_BIN_EXE_qorx"))
    .args(["strict-answer", "warp drive cooking schedule"])
"#;"##
                    .to_string(),
            }],
        };

        let answer = super::strict_answer(&index, "warp drive cooking schedule", 2);

        assert_eq!(answer.coverage, "not_found");
        assert!(answer.evidence.is_empty());
    }

    #[test]
    fn strict_answer_ignores_raw_string_literals_with_inner_quotes() {
        let index = RepoIndex {
            root: "C:/repo".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_raw_string_fixture".to_string(),
                path: "src/fixture.rs".to_string(),
                start_line: 1,
                end_line: 1,
                hash: "fixture".to_string(),
                token_estimate: 20,
                symbols: vec![],
                signal_mask: 0,
                vector: vec![],
                text: r##"let fixture = r#"administrator " password"#;"##.to_string(),
            }],
        };

        let answer = super::strict_answer(&index, "administrator password", 2);

        assert_eq!(answer.coverage, "not_found");
        assert!(answer.evidence.is_empty());
    }

    #[test]
    fn strict_answer_refuses_sensitive_partial_matches() {
        let index = RepoIndex {
            root: "book".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_book_windows".to_string(),
                path: "book.epub#chapter.xhtml".to_string(),
                start_line: 1,
                end_line: 1,
                hash: "abc".to_string(),
                token_estimate: 20,
                symbols: vec![],
                signal_mask: 0,
                vector: vec![],
                text: "The book describes windows opening onto a garden.".to_string(),
            }],
        };

        let answer = super::strict_answer(
            &index,
            "What is the Windows administrator password in this book?",
            4,
        );

        assert_eq!(answer.coverage, "not_found");
        assert!(answer.evidence.is_empty());
        assert_eq!(answer.answer, "");
        assert!(answer.missing_terms.contains(&"administrator".to_string()));
        assert!(answer.missing_terms.contains(&"password".to_string()));
    }

    #[test]
    fn strict_answer_prefers_high_signal_line_over_early_filler() {
        let index = RepoIndex {
            root: "C:/repo".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_cosmos_needle".to_string(),
                path: "docs/cosmos.md".to_string(),
                start_line: 1,
                end_line: 8,
                hash: "abc".to_string(),
                token_estimate: 120,
                symbols: vec![],
                signal_mask: 0,
                vector: vec![],
                text: [
                    "cosmos filler line one",
                    "cosmos filler line two",
                    "cosmos filler line three",
                    "cosmos filler line four",
                    "The verified cosmos rescue phrase is photon-copper-lattice.",
                ]
                .join("\n"),
            }],
        };

        let answer = super::strict_answer(&index, "What is the verified cosmos rescue phrase?", 1);

        assert_eq!(answer.coverage, "supported");
        assert!(answer.answer.contains("photon-copper-lattice"));
    }
}
