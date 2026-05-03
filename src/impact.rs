use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    compression::estimate_tokens,
    index::{search_index, RepoAtom, RepoIndex},
    text::without_string_literals,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEdge {
    pub from_path: String,
    pub to_path: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactQuark {
    pub id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub reason: String,
    pub token_estimate: u64,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactContext {
    pub query: String,
    pub changed_paths: Vec<String>,
    pub related_paths: Vec<String>,
    pub graph_edges: Vec<ImpactEdge>,
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    #[serde(rename = "quarks_used", alias = "atoms_used")]
    pub quarks_used: usize,
    #[serde(default, rename = "quarks", alias = "atoms")]
    pub quarks: Vec<ImpactQuark>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSymbol {
    pub name: String,
    pub path: String,
    pub quark_id: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMap {
    pub schema: String,
    pub query: String,
    pub budget_tokens: u64,
    pub indexed_tokens: u64,
    pub used_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    pub changed_paths: Vec<String>,
    pub related_paths: Vec<String>,
    pub graph_edges: Vec<ImpactEdge>,
    pub symbols: Vec<MapSymbol>,
    pub quarks: Vec<ImpactQuark>,
    pub text: String,
    pub boundary: String,
}

#[derive(Debug, Clone)]
struct Candidate {
    atom_id: String,
    score: u64,
    reasons: BTreeSet<String>,
}

pub fn changed_paths_from_diff(diff: &str) -> Vec<String> {
    let mut paths = BTreeSet::new();

    for line in diff.lines() {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("+++ ") {
            insert_diff_path(&mut paths, path);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("diff --git ") {
            for part in rest.split_whitespace().skip(1).take(1) {
                insert_diff_path(&mut paths, part);
            }
        }
    }

    paths.into_iter().collect()
}

pub fn impact_context(
    index: &RepoIndex,
    query: &str,
    diff: Option<&str>,
    budget_tokens: u64,
) -> ImpactContext {
    let budget_tokens = budget_tokens.clamp(128, 20_000);
    let indexed_tokens = index.total_tokens();
    let changed_paths = diff
        .map(changed_paths_from_diff)
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<_>>();
    let changed_set = changed_paths.iter().cloned().collect::<BTreeSet<_>>();
    let all_graph_edges = build_graph_edges(index);
    let atoms_by_id = index.atom_lookup();
    let mut related_paths = BTreeSet::new();
    let mut candidate_map = BTreeMap::<String, Candidate>::new();

    for atom in &index.atoms {
        if changed_set.contains(&atom.path) {
            add_candidate(&mut candidate_map, atom, 10_000, "changed");
        }
    }

    for edge in &all_graph_edges {
        if changed_set.contains(&edge.from_path) && !changed_set.contains(&edge.to_path) {
            related_paths.insert(edge.to_path.clone());
            add_atoms_by_path(
                &mut candidate_map,
                index,
                &edge.to_path,
                7_500,
                &format!("callee:{}", edge.symbol),
            );
        }
        if changed_set.contains(&edge.to_path) && !changed_set.contains(&edge.from_path) {
            related_paths.insert(edge.from_path.clone());
            add_atoms_by_path(
                &mut candidate_map,
                index,
                &edge.from_path,
                7_000,
                &format!("caller:{}", edge.symbol),
            );
        }
    }

    for hit in search_index(index, query, 64) {
        if let Some(atom) = atoms_by_id.get(hit.id.as_str()).copied() {
            if !changed_set.is_empty()
                && !changed_set.contains(&atom.path)
                && !related_paths.contains(&atom.path)
            {
                continue;
            }
            add_candidate(
                &mut candidate_map,
                atom,
                1_000 + hit.score,
                &format!("search:{}", hit.score),
            );
        }
    }

    let mut candidates = candidate_map.into_values().collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then(a.atom_id.cmp(&b.atom_id)));

    let mut used_tokens = estimate_tokens(query) + 48;
    let mut quarks = Vec::new();
    let mut text = format!(
        "# Qorx impact context\nquery: {query}\nbudget_tokens: {budget_tokens}\nindexed_tokens: {indexed_tokens}\n"
    );
    append_list(&mut text, "changed_paths", &changed_paths);
    let graph_edges = relevant_graph_edges(&all_graph_edges, &changed_set);
    let related_paths = related_paths.into_iter().collect::<Vec<_>>();
    append_list(&mut text, "related_paths", &related_paths);
    append_edges(&mut text, &graph_edges, &changed_set);

    for candidate in candidates {
        let Some(atom) = atoms_by_id.get(candidate.atom_id.as_str()).copied() else {
            continue;
        };
        let reason = candidate.reasons.into_iter().collect::<Vec<_>>().join(",");
        let header = atom_header(atom, &reason);
        let header_tokens = estimate_tokens(&header);
        let needed = header_tokens + atom.token_estimate;
        if used_tokens + needed > budget_tokens {
            continue;
        }
        used_tokens += needed;
        text.push('\n');
        text.push_str(&header);
        text.push('\n');
        text.push_str(&atom.text);
        text.push('\n');
        quarks.push(ImpactQuark {
            id: atom.id.clone(),
            path: atom.path.clone(),
            start_line: atom.start_line,
            end_line: atom.end_line,
            reason,
            token_estimate: atom.token_estimate,
            symbols: atom.symbols.clone(),
        });
    }

    let omitted_tokens = indexed_tokens.saturating_sub(used_tokens.min(indexed_tokens));
    let context_reduction_x = indexed_tokens.max(1) as f64 / used_tokens.max(1) as f64;
    ImpactContext {
        query: query.to_string(),
        changed_paths,
        related_paths,
        graph_edges,
        budget_tokens,
        indexed_tokens,
        used_tokens,
        omitted_tokens,
        context_reduction_x,
        quarks_used: quarks.len(),
        quarks,
        text,
    }
}

pub fn map_context(
    index: &RepoIndex,
    query: &str,
    diff: Option<&str>,
    budget_tokens: u64,
) -> RepoMap {
    let budget_tokens = budget_tokens.clamp(128, 20_000);
    let impact = impact_context(index, query, diff, budget_tokens);
    let selected_ids = impact
        .quarks
        .iter()
        .map(|quark| quark.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut symbols = Vec::new();
    for atom in &index.atoms {
        if !selected_ids.contains(atom.id.as_str()) {
            continue;
        }
        for symbol in &atom.symbols {
            symbols.push(MapSymbol {
                name: symbol.clone(),
                path: atom.path.clone(),
                quark_id: atom.id.clone(),
                start_line: atom.start_line,
                end_line: atom.end_line,
            });
        }
    }
    symbols.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then(a.path.cmp(&b.path))
            .then(a.quark_id.cmp(&b.quark_id))
    });
    symbols.dedup_by(|a, b| a.name == b.name && a.path == b.path);

    let mut text = format!(
        "# Qorx repo map\nquery: {query}\nbudget_tokens: {budget_tokens}\nindexed_tokens: {}\n",
        impact.indexed_tokens
    );
    append_list(&mut text, "changed_paths", &impact.changed_paths);
    append_list(&mut text, "related_paths", &impact.related_paths);
    text.push_str("symbols:\n");
    for symbol in &symbols {
        text.push_str(&format!(
            "- {} {}:{}-{} id={}\n",
            symbol.name, symbol.path, symbol.start_line, symbol.end_line, symbol.quark_id
        ));
    }
    text.push_str("graph_edges:\n");
    for edge in &impact.graph_edges {
        text.push_str(&format!(
            "- {} -> {} via {}\n",
            edge.from_path, edge.to_path, edge.symbol
        ));
    }
    text.push_str("quarks:\n");
    for quark in &impact.quarks {
        text.push_str(&format!(
            "- {}:{}-{} id={} reason={} tokens={}\n",
            quark.path,
            quark.start_line,
            quark.end_line,
            quark.id,
            quark.reason,
            quark.token_estimate
        ));
    }
    let used_tokens = estimate_tokens(&text).min(budget_tokens);
    let omitted_tokens = impact
        .indexed_tokens
        .saturating_sub(used_tokens.min(impact.indexed_tokens));
    let context_reduction_x = impact.indexed_tokens.max(1) as f64 / used_tokens.max(1) as f64;

    RepoMap {
        schema: "qorx.map.v1".to_string(),
        query: query.to_string(),
        budget_tokens,
        indexed_tokens: impact.indexed_tokens,
        used_tokens,
        omitted_tokens,
        context_reduction_x,
        changed_paths: impact.changed_paths,
        related_paths: impact.related_paths,
        graph_edges: impact.graph_edges,
        symbols,
        quarks: impact.quarks,
        text,
        boundary: "Repo map is a deterministic lightweight symbol and edge map over indexed quarks. It is not a full AST, Tree-sitter database, or semantic embedding index.".to_string(),
    }
}

fn insert_diff_path(paths: &mut BTreeSet<String>, raw_path: &str) {
    let path = raw_path.trim().trim_matches('"');
    if path == "/dev/null" || path.is_empty() {
        return;
    }
    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
        .replace('\\', "/");
    if !path.is_empty() && path != "/dev/null" {
        paths.insert(path);
    }
}

fn build_graph_edges(index: &RepoIndex) -> Vec<ImpactEdge> {
    let mut owners = BTreeMap::<String, BTreeSet<String>>::new();
    for atom in &index.atoms {
        if !is_code_path(&atom.path) {
            continue;
        }
        for symbol in &atom.symbols {
            if !is_graph_symbol(symbol) {
                continue;
            }
            owners
                .entry(symbol.to_string())
                .or_default()
                .insert(atom.path.clone());
        }
    }

    let mut edges = BTreeSet::<(String, String, String)>::new();
    for atom in &index.atoms {
        if !is_code_path(&atom.path) {
            continue;
        }
        for symbol in referenced_symbols(&atom.text) {
            let Some(paths) = owners.get(&symbol) else {
                continue;
            };
            for target_path in paths {
                if target_path == &atom.path {
                    continue;
                }
                edges.insert((atom.path.clone(), target_path.clone(), symbol.clone()));
            }
        }
    }

    edges
        .into_iter()
        .map(|(from_path, to_path, symbol)| ImpactEdge {
            from_path,
            to_path,
            symbol,
        })
        .collect()
}

fn relevant_graph_edges(edges: &[ImpactEdge], changed_paths: &BTreeSet<String>) -> Vec<ImpactEdge> {
    if changed_paths.is_empty() {
        return Vec::new();
    }

    edges
        .iter()
        .filter(|edge| {
            changed_paths.contains(&edge.from_path) || changed_paths.contains(&edge.to_path)
        })
        .take(64)
        .cloned()
        .collect()
}

fn referenced_symbols(text: &str) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();
    let code_text = without_string_literals(text);
    for token in identifier_tokens(&code_text) {
        if !is_graph_symbol(&token) {
            continue;
        }
        symbols.insert(token);
    }
    symbols
}

fn identifier_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            current.push(ch);
            continue;
        }
        push_identifier(&mut tokens, &mut current);
    }
    push_identifier(&mut tokens, &mut current);
    tokens
}

fn push_identifier(tokens: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    if current.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        current.clear();
        return;
    }
    tokens.push(std::mem::take(current));
}

fn is_code_path(path: &str) -> bool {
    let extension = path.rsplit('.').next().unwrap_or_default();
    matches!(
        extension,
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

fn is_graph_symbol(symbol: &str) -> bool {
    if symbol.len() <= 2 || is_reference_noise(symbol) {
        return false;
    }
    symbol.contains('_') || symbol.chars().any(|ch| ch.is_uppercase())
}

fn is_reference_noise(token: &str) -> bool {
    matches!(
        token,
        "Err"
            | "Json"
            | "None"
            | "Ok"
            | "Option"
            | "Result"
            | "Some"
            | "String"
            | "Value"
            | "Vec"
            | "as"
            | "async"
            | "await"
            | "body"
            | "budget_tokens"
            | "break"
            | "bytes"
            | "cache"
            | "cache_hit"
            | "class"
            | "cli"
            | "command"
            | "config"
            | "const"
            | "compressed_prompt_tokens"
            | "context_reduction_x"
            | "continue"
            | "crate"
            | "default"
            | "def"
            | "describe"
            | "else"
            | "enum"
            | "error"
            | "exe"
            | "expect"
            | "export"
            | "false"
            | "fn"
            | "for"
            | "from"
            | "function"
            | "if"
            | "impl"
            | "import"
            | "in"
            | "index"
            | "interface"
            | "indexed_tokens"
            | "line"
            | "load"
            | "let"
            | "match"
            | "mod"
            | "mut"
            | "new"
            | "null"
            | "omitted_tokens"
            | "path"
            | "provider_cache_write_tokens"
            | "provider_cached_prompt_tokens"
            | "pub"
            | "query"
            | "raw_prompt_tokens"
            | "read"
            | "report"
            | "request"
            | "result"
            | "return"
            | "self"
            | "server"
            | "source"
            | "status"
            | "struct"
            | "test"
            | "text"
            | "this"
            | "token_estimate"
            | "toBeTruthy"
            | "trait"
            | "true"
            | "type"
            | "undefined"
            | "upstream_error"
            | "use"
            | "used_tokens"
            | "while"
            | "write"
    )
}

fn add_atoms_by_path(
    candidates: &mut BTreeMap<String, Candidate>,
    index: &RepoIndex,
    path: &str,
    score: u64,
    reason: &str,
) {
    for atom in index.atoms.iter().filter(|atom| atom.path == path) {
        add_candidate(candidates, atom, score, reason);
    }
}

fn add_candidate(
    candidates: &mut BTreeMap<String, Candidate>,
    atom: &RepoAtom,
    score: u64,
    reason: &str,
) {
    let entry = candidates
        .entry(atom.id.clone())
        .or_insert_with(|| Candidate {
            atom_id: atom.id.clone(),
            score: 0,
            reasons: BTreeSet::new(),
        });
    entry.score = entry.score.max(score);
    entry.reasons.insert(reason.to_string());
}

fn append_list(text: &mut String, label: &str, paths: &[String]) {
    text.push_str(label);
    text.push_str(": ");
    if paths.is_empty() {
        text.push_str("[]\n");
    } else {
        text.push_str(&paths.join(", "));
        text.push('\n');
    }
}

fn append_edges(text: &mut String, edges: &[ImpactEdge], changed_paths: &BTreeSet<String>) {
    let relevant = edges
        .iter()
        .filter(|edge| {
            changed_paths.contains(&edge.from_path) || changed_paths.contains(&edge.to_path)
        })
        .take(64)
        .collect::<Vec<_>>();
    text.push_str("graph_edges: ");
    if relevant.is_empty() {
        text.push_str("[]\n");
        return;
    }
    text.push('\n');
    for edge in relevant {
        text.push_str("- ");
        text.push_str(&edge.from_path);
        text.push_str(" -> ");
        text.push_str(&edge.to_path);
        text.push_str(" via ");
        text.push_str(&edge.symbol);
        text.push('\n');
    }
}

fn atom_header(atom: &RepoAtom, reason: &str) -> String {
    let mut header = format!(
        "## {}:{}-{} id={} reason={} tokens={}",
        atom.path, atom.start_line, atom.end_line, atom.id, reason, atom.token_estimate
    );
    if !atom.symbols.is_empty() {
        header.push_str(" symbols=");
        header.push_str(&atom.symbols.join(","));
    }
    header
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::index::{RepoAtom, RepoIndex};

    use super::{changed_paths_from_diff, impact_context, referenced_symbols};

    fn atom(id: &str, path: &str, symbols: &[&str], text: &str) -> RepoAtom {
        RepoAtom {
            id: id.to_string(),
            path: path.to_string(),
            start_line: 1,
            end_line: text.lines().count().max(1),
            hash: format!("hash_{id}"),
            token_estimate: 20,
            symbols: symbols.iter().map(|symbol| symbol.to_string()).collect(),
            signal_mask: 0,
            vector: Vec::new(),
            text: text.to_string(),
        }
    }

    fn fixture_index() -> RepoIndex {
        RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![
                atom(
                    "qva_route",
                    "src/routes/auth.ts",
                    &["postLogin"],
                    "import { issueSession } from '../services/session';\nimport { auditEvent } from '../services/audit';\nexport function postLogin(req) {\n  issueSession(req.user.id);\n  auditEvent('login');\n}",
                ),
                atom(
                    "qva_session",
                    "src/services/session.ts",
                    &["issueSession"],
                    "export function issueSession(userId) {\n  return { userId, expiresIn: 3600 };\n}",
                ),
                atom(
                    "qva_audit",
                    "src/services/audit.ts",
                    &["auditEvent"],
                    "export function auditEvent(name) {\n  return { name };\n}",
                ),
                atom(
                    "qva_test",
                    "tests/auth.test.ts",
                    &[],
                    "import { postLogin } from '../src/routes/auth';\ntest('login creates a session', () => {\n  expect(postLogin({ user: { id: 'u1' } })).toBeTruthy();\n});",
                ),
                atom(
                    "qva_billing",
                    "src/services/billing.ts",
                    &["chargeCard"],
                    "export function chargeCard() {\n  return 'unrelated';\n}",
                ),
                atom(
                    "qva_report_route",
                    "src/routes/report.ts",
                    &["getReport"],
                    "import { renderReport } from '../services/report';\nexport function getReport() {\n  return renderReport();\n}",
                ),
                atom(
                    "qva_report_service",
                    "src/services/report.ts",
                    &["renderReport"],
                    "export function renderReport() {\n  return 'report';\n}",
                ),
            ],
        }
    }

    #[test]
    fn diff_parser_reads_git_headers_and_new_files() {
        let diff = "\
diff --git a/src/routes/auth.ts b/src/routes/auth.ts
index 111..222 100644
--- a/src/routes/auth.ts
+++ b/src/routes/auth.ts
@@ -1 +1 @@
-old
+new
diff --git a/src/new.ts b/src/new.ts
new file mode 100644
--- /dev/null
+++ b/src/new.ts
@@ -0,0 +1 @@
+created
";

        let paths = changed_paths_from_diff(diff);
        assert_eq!(paths, vec!["src/new.ts", "src/routes/auth.ts"]);
    }

    #[test]
    fn impact_context_links_changed_file_to_services_and_tests() {
        let index = fixture_index();
        let diff = "\
diff --git a/src/routes/auth.ts b/src/routes/auth.ts
--- a/src/routes/auth.ts
+++ b/src/routes/auth.ts
@@ -1 +1 @@
-old
+new
";

        let impact = impact_context(&index, "login session behavior", Some(diff), 512);

        assert_eq!(impact.changed_paths, vec!["src/routes/auth.ts"]);
        assert!(impact
            .related_paths
            .contains(&"src/services/session.ts".to_string()));
        assert!(impact
            .related_paths
            .contains(&"src/services/audit.ts".to_string()));
        assert!(impact
            .related_paths
            .contains(&"tests/auth.test.ts".to_string()));

        let selected = impact
            .quarks
            .iter()
            .map(|quark| quark.id.as_str())
            .collect::<Vec<_>>();
        assert!(selected.contains(&"qva_route"));
        assert!(selected.contains(&"qva_session"));
        assert!(selected.contains(&"qva_audit"));
        assert!(selected.contains(&"qva_test"));
        assert!(!selected.contains(&"qva_billing"));
        assert!(impact
            .graph_edges
            .iter()
            .all(|edge| edge.from_path.contains("auth")
                || edge.to_path.contains("auth")
                || impact.related_paths.contains(&edge.from_path)
                || impact.related_paths.contains(&edge.to_path)));
        assert!(!impact
            .graph_edges
            .iter()
            .any(|edge| edge.symbol == "renderReport"));
        assert!(impact.text.contains("issueSession"));
        assert!(impact.context_reduction_x > 0.0);
    }

    #[test]
    fn reference_extraction_ignores_symbols_inside_string_literals() {
        let refs = referenced_symbols("let fixture = \"postLogin issueSession\";\npostLogin(req);");

        assert!(refs.contains("postLogin"));
        assert!(!refs.contains("issueSession"));
    }

    #[test]
    fn reference_extraction_ignores_raw_strings_with_inner_quotes() {
        let refs =
            referenced_symbols("let fixture = r#\"postLogin \" issueSession\"#;\npostLogin(req);");

        assert!(refs.contains("postLogin"));
        assert!(!refs.contains("issueSession"));
    }

    #[test]
    fn impact_context_falls_back_to_search_without_diff() {
        let index = RepoIndex {
            root: "test".to_string(),
            updated_at: Utc::now(),
            atoms: vec![atom(
                "qva_cache",
                "src/response_cache.rs",
                &["ExactResponseCache"],
                "pub struct ExactResponseCache { hits: u64 }",
            )],
        };

        let impact = impact_context(&index, "ExactResponseCache", None, 256);

        assert!(impact.changed_paths.is_empty());
        assert_eq!(impact.quarks_used, 1);
        assert_eq!(impact.quarks[0].id, "qva_cache");
        assert!(impact.text.contains("ExactResponseCache"));
    }
}
