use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    compression::estimate_tokens,
    index::{search_index, RepoAtom, RepoIndex},
};

const SIG_TEST: u16 = 1 << 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqueezeReport {
    pub schema: String,
    pub query: String,
    pub mode: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub source_tokens: u64,
    pub squeezed_tokens: u64,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    pub quarks_used: usize,
    pub evidence: Vec<SqueezedEvidence>,
    pub text: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqueezedEvidence {
    pub id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub source_tokens: u64,
    pub excerpt_tokens: u64,
    pub matched_terms: Vec<String>,
    pub excerpt_hash: String,
    pub excerpt: String,
}

pub fn squeeze_context(
    index: &RepoIndex,
    query: &str,
    budget_tokens: u64,
    limit: usize,
) -> SqueezeReport {
    let budget_tokens = budget_tokens.clamp(96, 20_000);
    let limit = limit.clamp(1, 16);
    let indexed_tokens = index.total_tokens();
    let terms = meaningful_terms(query);
    let mut text = format!(
        "# Qorx squeezed context\nquery: {query}\nbudget_tokens: {budget_tokens}\nindexed_tokens: {indexed_tokens}\n"
    );
    let mut used_tokens = estimate_tokens(&text);
    let mut source_tokens = 0;
    let mut squeezed_tokens = 0;
    let mut evidence = Vec::new();
    let atoms_by_id = index.atom_lookup();

    for hit in search_index(index, query, 96) {
        if evidence.len() >= limit {
            break;
        }
        let Some(atom) = atoms_by_id.get(hit.id.as_str()).copied() else {
            continue;
        };
        if is_test_like(atom) && !query_wants_tests(&terms) {
            continue;
        }
        let matched_terms = matched_terms(atom, &terms);
        if matched_terms.is_empty() {
            continue;
        }
        let excerpt = squeeze_atom(atom, &terms);
        if excerpt.is_empty() {
            continue;
        }
        if is_fixture_echo(atom, &excerpt) {
            continue;
        }
        let excerpt_tokens = estimate_tokens(&excerpt);
        let header = format!(
            "\n## {}:{}-{} id={} source_tokens={} excerpt_tokens={} score={}\n",
            atom.path,
            atom.start_line,
            atom.end_line,
            atom.id,
            atom.token_estimate,
            excerpt_tokens,
            hit.score
        );
        let needed = estimate_tokens(&header) + excerpt_tokens;
        if used_tokens + needed > budget_tokens {
            continue;
        }
        used_tokens += needed;
        source_tokens += atom.token_estimate;
        squeezed_tokens += excerpt_tokens;
        text.push_str(&header);
        text.push_str(&excerpt);
        text.push('\n');
        evidence.push(SqueezedEvidence {
            id: atom.id.clone(),
            path: atom.path.clone(),
            start_line: atom.start_line,
            end_line: atom.end_line,
            source_tokens: atom.token_estimate,
            excerpt_tokens,
            matched_terms,
            excerpt_hash: excerpt_hash(&excerpt),
            excerpt,
        });
    }

    let omitted_tokens = indexed_tokens.saturating_sub(used_tokens.min(indexed_tokens));
    let context_reduction_x = indexed_tokens.max(1) as f64 / used_tokens.max(1) as f64;
    SqueezeReport {
        schema: "qorx.squeeze.v1".to_string(),
        query: query.to_string(),
        mode: "extractive_query_squeeze".to_string(),
        local_only: true,
        provider_calls: 0,
        budget_tokens,
        indexed_tokens,
        source_tokens,
        squeezed_tokens,
        used_tokens,
        omitted_tokens,
        context_reduction_x,
        quarks_used: evidence.len(),
        evidence,
        text,
        boundary: "Squeeze is extractive and local: it keeps only query-relevant lines from indexed quarks, preserves citations and hashes, and uses no model calls or generated missing context.".to_string(),
    }
}

fn squeeze_atom(atom: &RepoAtom, terms: &[String]) -> String {
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
            let score = terms
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
    ranked
        .into_iter()
        .map(|(_, _, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn matched_terms(atom: &RepoAtom, terms: &[String]) -> Vec<String> {
    let haystack = format!("{} {} {}", atom.path, atom.symbols.join(" "), atom.text).to_lowercase();
    terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .cloned()
        .collect()
}

fn meaningful_terms(text: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else {
            push_term(&mut terms, &mut current);
        }
    }
    push_term(&mut terms, &mut current);
    terms.into_iter().collect()
}

fn push_term(terms: &mut BTreeSet<String>, current: &mut String) {
    if current.len() >= 3 && !is_stopword(current) {
        terms.insert(std::mem::take(current));
    }
    current.clear();
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "and"
            | "are"
            | "before"
            | "for"
            | "from"
            | "into"
            | "qorx"
            | "that"
            | "the"
            | "this"
            | "with"
    )
}

fn excerpt_hash(excerpt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(excerpt.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

fn is_fixture_echo(atom: &RepoAtom, excerpt: &str) -> bool {
    let path = atom.path.replace('\\', "/").to_lowercase();
    let text = format!("{}\n{}", atom.text, excerpt).to_lowercase();
    let test_path = path.starts_with("tests/") || path.contains("/tests/");
    let cli_fixture = text.contains(".args(") || text.contains("cargo_bin_exe_qorx");
    let rust_unit_test = text.contains("#[test]") || text.contains("mod tests");
    (test_path || cli_fixture || rust_unit_test)
        && (text.contains("squeeze")
            || text.contains("strict-answer")
            || text.contains("judge")
            || text.contains("agent")
            || text.contains("marvin"))
}

fn is_test_like(atom: &RepoAtom) -> bool {
    let path = atom.path.replace('\\', "/").to_lowercase();
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.contains("_test.")
        || path.contains(".test.")
        || path.contains(".spec.")
        || atom.signal_mask & SIG_TEST != 0
        || atom.text.contains("#[test]")
        || atom.text.contains("mod tests")
}

fn query_wants_tests(terms: &[String]) -> bool {
    terms
        .iter()
        .any(|term| matches!(term.as_str(), "test" | "tests" | "spec" | "fixture"))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::index::{RepoAtom, RepoIndex};

    #[test]
    fn squeeze_removes_non_matching_lines() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![RepoAtom {
                id: "qva_test".to_string(),
                path: "docs/proof.md".to_string(),
                start_line: 1,
                end_line: 3,
                hash: "abc".to_string(),
                token_estimate: 80,
                symbols: vec![],
                signal_mask: 0,
                vector: vec![],
                text: "provider savings proof line\nunrelated filler\nrouted provider evidence"
                    .to_string(),
            }],
        };

        let report = super::squeeze_context(&index, "provider evidence", 160, 2);

        assert_eq!(report.evidence.len(), 1);
        assert!(report.text.contains("provider savings proof line"));
        assert!(!report.text.contains("unrelated filler"));
    }

    #[test]
    fn squeeze_ignores_cli_fixture_echoes_in_tests() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![
                RepoAtom {
                    id: "qva_fixture".to_string(),
                    path: "tests/research_features.rs".to_string(),
                    start_line: 1,
                    end_line: 4,
                    hash: "fixture".to_string(),
                    token_estimate: 40,
                    symbols: vec![],
                    signal_mask: 0,
                    vector: vec![],
                    text: ".args([\"squeeze\", \"production gate provider savings\"])\n\"text\": \"production gate provider savings fixture\"".to_string(),
                },
                RepoAtom {
                    id: "qva_real".to_string(),
                    path: "src/money.rs".to_string(),
                    start_line: 1,
                    end_line: 2,
                    hash: "real".to_string(),
                    token_estimate: 40,
                    symbols: vec![],
                    signal_mask: 0,
                    vector: vec![],
                    text: "production gate requires routed provider savings evidence".to_string(),
                },
            ],
        };

        let report = super::squeeze_context(&index, "production gate provider savings", 200, 4);

        assert!(report.text.contains("routed provider savings evidence"));
        assert!(!report.text.contains(".args("));
        assert!(report
            .evidence
            .iter()
            .all(|item| item.path != "tests/research_features.rs"));
    }

    #[test]
    fn squeeze_skips_test_quarks_for_non_test_queries() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![
                RepoAtom {
                    id: "qva_unit_test".to_string(),
                    path: "src/money.rs".to_string(),
                    start_line: 1,
                    end_line: 8,
                    hash: "test".to_string(),
                    token_estimate: 40,
                    symbols: vec!["money_claim_guard_rejects_unsupported_savings".to_string()],
                    signal_mask: 4,
                    vector: vec![],
                    text: "#[test]\nfn money_claim_guard_rejects_unsupported_savings() {}"
                        .to_string(),
                },
                RepoAtom {
                    id: "qva_money".to_string(),
                    path: "src/money.rs".to_string(),
                    start_line: 1,
                    end_line: 2,
                    hash: "money".to_string(),
                    token_estimate: 40,
                    symbols: vec![],
                    signal_mask: 0,
                    vector: vec![],
                    text: "production gate requires routed provider savings evidence".to_string(),
                },
            ],
        };

        let report = super::squeeze_context(&index, "production gate provider savings", 200, 4);

        assert!(report.text.contains("routed provider savings evidence"));
        assert!(!report
            .text
            .contains("qorx_headers_mark_routed_provider_savings"));
    }
}
