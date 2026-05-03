use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    compression::estimate_tokens,
    index::{search_index, RepoAtom, RepoIndex},
};

const SCHEMA: &str = "qorx.b2c-plan.v1";
const RISK_CAP: f64 = 0.78;
const LANE_NAMES: [&str; 5] = ["retrieval", "portfolio", "risk", "cache", "carrier"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2cPlan {
    pub schema: String,
    pub query: String,
    pub local_only: bool,
    pub provider_calls: u64,
    pub route: String,
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    pub selected_quarks: Vec<B2cSelectedQuark>,
    pub rejected_quarks: Vec<B2cRejectedQuark>,
    pub parallel_lanes: Vec<B2cLane>,
    pub math: B2cMath,
    pub text: String,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2cSelectedQuark {
    pub id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub token_estimate: u64,
    pub retrieval_score: u64,
    pub expected_value: f64,
    pub token_cost: f64,
    pub redundancy_penalty: f64,
    pub omission_risk: f64,
    pub cache_value: f64,
    pub net_value: f64,
    pub carrier: String,
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2cRejectedQuark {
    pub id: String,
    pub path: String,
    pub token_estimate: u64,
    pub net_value: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2cLane {
    pub name: String,
    pub mode: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2cMath {
    pub budget_model: String,
    pub redundancy_model: String,
    pub risk_model: String,
    pub cache_model: String,
    pub score_formula: String,
}

#[derive(Debug, Clone)]
struct Candidate {
    atom: RepoAtom,
    retrieval_score: u64,
    expected_value: f64,
    token_cost: f64,
    omission_risk: f64,
    cache_value: f64,
    matched_terms: Vec<String>,
    carrier: String,
}

#[derive(Debug, Clone)]
struct Pick {
    candidate: Candidate,
    redundancy_penalty: f64,
    net_value: f64,
    needed_tokens: u64,
}

pub fn plan_context(index: &RepoIndex, query: &str, budget_tokens: u64) -> B2cPlan {
    let budget_tokens = budget_tokens.clamp(128, 20_000);
    let indexed_tokens = index.total_tokens();
    let terms = meaningful_terms(query);
    let candidates = candidates(index, query, &terms);

    let mut remaining = candidates;
    let mut picks: Vec<Pick> = Vec::new();
    let mut rejected = Vec::new();
    let mut used_tokens = estimate_tokens(query) + 8;

    loop {
        let mut best: Option<(usize, Pick, f64)> = None;
        for (idx, candidate) in remaining.iter().enumerate() {
            let redundancy_penalty = redundancy_penalty(candidate, &picks);
            let net_value = net_value(candidate, redundancy_penalty);
            let needed_tokens = candidate_tokens(candidate);
            if used_tokens + needed_tokens > budget_tokens {
                continue;
            }
            if net_value <= 0.0 {
                continue;
            }
            if candidate.omission_risk > RISK_CAP && picks.is_empty() {
                continue;
            }
            let utility_density = net_value / needed_tokens.max(1) as f64;
            let pick = Pick {
                candidate: candidate.clone(),
                redundancy_penalty,
                net_value,
                needed_tokens,
            };
            match &best {
                Some((_, _, best_density)) if *best_density >= utility_density => {}
                _ => best = Some((idx, pick, utility_density)),
            }
        }

        let Some((idx, pick, _)) = best else {
            break;
        };
        used_tokens += pick.needed_tokens;
        picks.push(pick);
        remaining.remove(idx);
    }

    for candidate in remaining.into_iter().take(16) {
        let redundancy_penalty = redundancy_penalty(&candidate, &picks);
        let net_value = net_value(&candidate, redundancy_penalty);
        let reason = if candidate_tokens(&candidate) + used_tokens > budget_tokens {
            "budget_exceeded"
        } else if candidate.omission_risk > RISK_CAP {
            "omission_risk_above_cap"
        } else if net_value <= 0.0 {
            "non_positive_net_value"
        } else {
            "lower_utility_density"
        };
        rejected.push(B2cRejectedQuark {
            id: candidate.atom.id,
            path: candidate.atom.path,
            token_estimate: candidate.atom.token_estimate,
            net_value: round2(net_value),
            reason: reason.to_string(),
        });
    }

    let selected_quarks = picks.iter().map(selected_quark).collect::<Vec<_>>();
    let route = choose_route(&selected_quarks, used_tokens, budget_tokens);
    let omitted_tokens = indexed_tokens.saturating_sub(used_tokens.min(indexed_tokens));
    let context_reduction_x = indexed_tokens.max(1) as f64 / used_tokens.max(1) as f64;
    let text = render_context_text(query, budget_tokens, indexed_tokens, &route, &picks);

    B2cPlan {
        schema: SCHEMA.to_string(),
        query: query.to_string(),
        local_only: true,
        provider_calls: 0,
        route: route.clone(),
        budget_tokens,
        indexed_tokens,
        used_tokens,
        omitted_tokens,
        context_reduction_x,
        selected_quarks,
        rejected_quarks: rejected,
        parallel_lanes: lanes(&route, &terms, &picks),
        math: B2cMath {
            budget_model: "bounded_knapsack".to_string(),
            redundancy_model: "portfolio_diversification".to_string(),
            risk_model: "omission_risk_cap".to_string(),
            cache_model: "stable_quark_reuse_value".to_string(),
            score_formula: "net=expected_value + cache_value - token_cost - redundancy_penalty - omission_risk_penalty".to_string(),
        },
        text,
        boundary: "B2C quant planning is deterministic local math over indexed quarks: sparse retrieval, budgeted selection, redundancy penalties, risk caps, and cache value. It performs no provider calls and makes no dollar claim without Qorx stats.".to_string(),
    }
}

fn candidates(index: &RepoIndex, query: &str, terms: &[String]) -> Vec<Candidate> {
    let atoms = index.atom_lookup();
    let mut out = Vec::new();
    for hit in search_index(index, query, 128) {
        let Some(atom) = atoms.get(hit.id.as_str()).copied() else {
            continue;
        };
        let matched_terms = matched_terms(atom, terms);
        let coverage = if terms.is_empty() {
            0.0
        } else {
            matched_terms.len() as f64 / terms.len() as f64
        };
        let symbol_bonus = if atom.symbols.is_empty() { 0.0 } else { 6.0 };
        let structural_bonus = if atom.signal_mask == 0 { 0.0 } else { 4.0 };
        let expected_value =
            (hit.score as f64).ln_1p() * 12.0 + coverage * 48.0 + symbol_bonus + structural_bonus;
        let token_cost = (atom.token_estimate as f64).sqrt() * 1.7;
        let omission_risk = (1.0 - coverage).clamp(0.05, 0.95);
        let cache_value = stable_quark_cache_value(atom);
        let carrier = carrier_for(atom, omission_risk);
        out.push(Candidate {
            atom: atom.clone(),
            retrieval_score: hit.score,
            expected_value,
            token_cost,
            omission_risk,
            cache_value,
            matched_terms,
            carrier,
        });
    }
    out.sort_by(|a, b| {
        b.retrieval_score
            .cmp(&a.retrieval_score)
            .then(a.atom.path.cmp(&b.atom.path))
    });
    out
}

fn candidate_tokens(candidate: &Candidate) -> u64 {
    candidate.atom.token_estimate + estimate_tokens(&short_header(candidate))
}

fn selected_quark(pick: &Pick) -> B2cSelectedQuark {
    let candidate = &pick.candidate;
    B2cSelectedQuark {
        id: candidate.atom.id.clone(),
        path: candidate.atom.path.clone(),
        start_line: candidate.atom.start_line,
        end_line: candidate.atom.end_line,
        token_estimate: candidate.atom.token_estimate,
        retrieval_score: candidate.retrieval_score,
        expected_value: round2(candidate.expected_value),
        token_cost: round2(candidate.token_cost),
        redundancy_penalty: round2(pick.redundancy_penalty),
        omission_risk: round2(candidate.omission_risk),
        cache_value: round2(candidate.cache_value),
        net_value: round2(pick.net_value),
        carrier: candidate.carrier.clone(),
        matched_terms: candidate.matched_terms.clone(),
    }
}

fn net_value(candidate: &Candidate, redundancy_penalty: f64) -> f64 {
    let risk_penalty = candidate.omission_risk * 9.0;
    candidate.expected_value + candidate.cache_value
        - candidate.token_cost
        - redundancy_penalty
        - risk_penalty
}

fn redundancy_penalty(candidate: &Candidate, picks: &[Pick]) -> f64 {
    picks
        .iter()
        .map(|pick| pair_redundancy(candidate, &pick.candidate))
        .fold(0.0, f64::max)
}

fn pair_redundancy(left: &Candidate, right: &Candidate) -> f64 {
    let mut penalty = 0.0;
    if left.atom.path == right.atom.path {
        penalty += 22.0;
    } else if parent_dir(&left.atom.path) == parent_dir(&right.atom.path) {
        penalty += 4.0;
    }
    if !left.atom.symbols.is_empty()
        && left
            .atom
            .symbols
            .iter()
            .any(|symbol| right.atom.symbols.contains(symbol))
    {
        penalty += 12.0;
    }
    penalty += jaccard(&left.matched_terms, &right.matched_terms) * 16.0;
    penalty += vector_overlap(&left.atom.vector, &right.atom.vector).min(12) as f64 * 0.5;
    penalty
}

fn stable_quark_cache_value(atom: &RepoAtom) -> f64 {
    let stable_path = !(atom.path.contains("/tmp/")
        || atom.path.contains("\\tmp\\")
        || atom.path.contains("fixtures")
        || atom.path.contains("snapshot"));
    if stable_path {
        (atom.token_estimate as f64).ln_1p().min(8.0)
    } else {
        0.0
    }
}

fn carrier_for(atom: &RepoAtom, omission_risk: f64) -> String {
    if atom.token_estimate > 900 {
        "squeeze".to_string()
    } else if omission_risk <= 0.25 {
        "pack".to_string()
    } else {
        "fault_if_needed".to_string()
    }
}

fn choose_route(selected: &[B2cSelectedQuark], used_tokens: u64, budget_tokens: u64) -> String {
    if selected.is_empty() {
        return "fault".to_string();
    }
    let avg_risk =
        selected.iter().map(|item| item.omission_risk).sum::<f64>() / selected.len() as f64;
    let pressure = used_tokens as f64 / budget_tokens.max(1) as f64;
    if avg_risk <= 0.22 && pressure <= 0.35 {
        "handle".to_string()
    } else if selected.iter().any(|item| item.carrier == "squeeze") {
        "squeeze".to_string()
    } else {
        "pack".to_string()
    }
}

fn lanes(route: &str, terms: &[String], picks: &[Pick]) -> Vec<B2cLane> {
    let lane_output = [
        format!("{} query terms scored with sparse local retrieval", terms.len()),
        format!("{} quarks selected after redundancy penalties", picks.len()),
        format!("risk cap {:.2} applied before carrier selection", RISK_CAP),
        "stable indexed quarks receive reuse value; provider cache still requires upstream metadata"
            .to_string(),
        format!("route={route}"),
    ];

    LANE_NAMES
        .iter()
        .zip(lane_output)
        .map(|(name, output)| B2cLane {
            name: (*name).to_string(),
            mode: match *name {
                "retrieval" => "sparse_vector_and_lexical_score",
                "portfolio" => "budgeted_value_density_with_diversification",
                "risk" => "omission_risk_cap",
                "cache" => "stable_quark_reuse_value",
                _ => "carrier_decision",
            }
            .to_string(),
            output,
        })
        .collect()
}

fn render_context_text(
    query: &str,
    budget_tokens: u64,
    indexed_tokens: u64,
    route: &str,
    picks: &[Pick],
) -> String {
    let mut text = format!(
        "# Qorx B2C packed context\nquery: {query}\nbudget_tokens: {budget_tokens}\nindexed_tokens: {indexed_tokens}\nb2c_route={route}\nb2c_parallel_lanes={}\nb2c_math=bounded_knapsack,portfolio_diversification,omission_risk_cap,stable_quark_reuse_value\n",
        LANE_NAMES.join(",")
    );

    for pick in picks {
        let candidate = &pick.candidate;
        text.push('\n');
        text.push_str(&short_header(candidate));
        text.push_str(&format!(
            " b2c_net={:.2} retrieval={} risk={:.2} cache={:.2} carrier={}",
            pick.net_value,
            candidate.retrieval_score,
            candidate.omission_risk,
            candidate.cache_value,
            candidate.carrier
        ));
        text.push('\n');
        text.push_str(&candidate.atom.text);
        text.push('\n');
    }

    text
}

fn short_header(candidate: &Candidate) -> String {
    let atom = &candidate.atom;
    let mut header = format!(
        "## id={} path={} lines={}-{} tokens={}",
        atom.id, atom.path, atom.start_line, atom.end_line, atom.token_estimate
    );
    if !atom.symbols.is_empty() {
        header.push_str(" symbols=");
        header.push_str(&atom.symbols.join(","));
    }
    header
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
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
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
        "and" | "are" | "for" | "from" | "into" | "qorx" | "that" | "the" | "this" | "with"
    )
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once(['/', '\\'])
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

fn jaccard(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let left = left.iter().collect::<HashSet<_>>();
    let right = right.iter().collect::<HashSet<_>>();
    let intersection = left.intersection(&right).count() as f64;
    let union = left.union(&right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn vector_overlap(left: &[u32], right: &[u32]) -> u64 {
    let mut score = 0;
    let mut li = 0;
    let mut ri = 0;
    while li < left.len() && ri < right.len() {
        match left[li].cmp(&right[ri]) {
            std::cmp::Ordering::Equal => {
                score += 1;
                li += 1;
                ri += 1;
            }
            std::cmp::Ordering::Less => li += 1,
            std::cmp::Ordering::Greater => ri += 1,
        }
    }
    score
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn planner_prefers_diverse_relevant_quarks_under_budget() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![
                atom(
                    "a",
                    "src/auth.ts",
                    48,
                    "login route session audit",
                    &["loginRoute"],
                ),
                atom(
                    "b",
                    "src/session.ts",
                    36,
                    "issue session token for login",
                    &["issueSession"],
                ),
                atom(
                    "c",
                    "src/billing.ts",
                    28,
                    "charge invoice renewal",
                    &["charge"],
                ),
            ],
        };

        let plan = plan_context(&index, "login route session", 180);

        assert_eq!(plan.schema, SCHEMA);
        assert!(plan.selected_quarks.iter().any(|item| item.id == "a"));
        assert!(plan.selected_quarks.iter().any(|item| item.id == "b"));
        assert!(!plan.selected_quarks.iter().any(|item| item.id == "c"));
        assert_eq!(plan.provider_calls, 0);
    }

    fn atom(id: &str, path: &str, tokens: u64, text: &str, symbols: &[&str]) -> RepoAtom {
        RepoAtom {
            id: id.to_string(),
            path: path.to_string(),
            start_line: 1,
            end_line: 3,
            hash: id.to_string(),
            token_estimate: tokens,
            symbols: symbols.iter().map(|symbol| (*symbol).to_string()).collect(),
            signal_mask: 64,
            vector: vec![],
            text: text.to_string(),
        }
    }
}
