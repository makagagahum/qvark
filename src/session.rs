use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    compression::{estimate_tokens, TOKEN_ESTIMATOR_LABEL},
    cost_stack,
    index::RepoIndex,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPointer {
    pub handle: String,
    pub root: String,
    pub root_fingerprint: String,
    pub updated_at: String,
    #[serde(alias = "atom_count")]
    pub quark_count: usize,
    pub indexed_tokens: u64,
    pub visible_tokens: u64,
    pub omitted_tokens: u64,
    pub context_reduction_x: f64,
    pub boundary: String,
    pub prompt_block: String,
}

struct PromptBlockParts<'a> {
    handle: &'a str,
    root_short: &'a str,
    quark_count: usize,
    indexed_tokens: u64,
    updated_at: &'a str,
    visible_tokens: u64,
    omitted_tokens: u64,
    context_reduction_x: f64,
}

pub fn build_session_pointer(index: &RepoIndex) -> SessionPointer {
    let indexed_tokens = index.total_tokens();
    let root_fingerprint = root_fingerprint(index);
    let handle = format!("qorx://s/{}", &root_fingerprint[..16]);
    let boundary =
        "Qorx session pointers are local resolver handles; exact context stays in local Qorx qosm state and is resolved by the Qorx proxy. qosm is the Qorx name for local protobuf state, not a physics claim."
            .to_string();
    let root_short = &root_fingerprint[..16];
    let updated_at = index
        .updated_at
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut visible_tokens = 1;
    let mut omitted_tokens = indexed_tokens;
    let mut context_reduction_x = indexed_tokens.max(1) as f64;
    let mut prompt_block = String::new();
    for _ in 0..4 {
        prompt_block = build_prompt_block(PromptBlockParts {
            handle: &handle,
            root_short,
            quark_count: index.atoms.len(),
            indexed_tokens,
            updated_at: &updated_at,
            visible_tokens,
            omitted_tokens,
            context_reduction_x,
        });
        let next_visible_tokens = estimate_tokens(&prompt_block).max(1);
        let next_omitted_tokens = indexed_tokens.saturating_sub(next_visible_tokens);
        let next_context_reduction_x =
            indexed_tokens.max(1) as f64 / next_visible_tokens.max(1) as f64;
        if next_visible_tokens == visible_tokens {
            break;
        }
        visible_tokens = next_visible_tokens;
        omitted_tokens = next_omitted_tokens;
        context_reduction_x = next_context_reduction_x;
    }
    let visible_tokens = estimate_tokens(&prompt_block).max(1);
    let omitted_tokens = indexed_tokens.saturating_sub(visible_tokens);
    let context_reduction_x = indexed_tokens.max(1) as f64 / visible_tokens.max(1) as f64;

    SessionPointer {
        handle,
        root: index.root.clone(),
        root_fingerprint,
        updated_at: index.updated_at.to_rfc3339(),
        quark_count: index.atoms.len(),
        indexed_tokens,
        visible_tokens,
        omitted_tokens,
        context_reduction_x,
        boundary,
        prompt_block,
    }
}

fn build_prompt_block(parts: PromptBlockParts<'_>) -> String {
    let PromptBlockParts {
        handle,
        root_short,
        quark_count,
        indexed_tokens,
        updated_at,
        visible_tokens,
        omitted_tokens,
        context_reduction_x,
    } = parts;
    format!(
        "QORX_SESSION {handle}\nr={root_short} q={quark_count} local_idx={indexed_tokens}\nqosm=local qshf=ready; local_idx stays local; resolve with Qorx.\nproof at={updated_at} ctx={indexed_tokens}t vis={visible_tokens}t omitted={omitted_tokens}t qshf={context_reduction_x:.2}x est={TOKEN_ESTIMATOR_LABEL} b2c=accounting {stack}",
        stack = cost_stack::PROMPT_TAG,
    )
}

fn root_fingerprint(index: &RepoIndex) -> String {
    let mut hasher = Sha256::new();
    hasher.update(index.root.as_bytes());
    hasher.update(index.updated_at.to_rfc3339().as_bytes());
    for atom in &index.atoms {
        hasher.update(atom.id.as_bytes());
        hasher.update(atom.hash.as_bytes());
        hasher.update(atom.path.as_bytes());
        hasher.update(atom.start_line.to_le_bytes());
        hasher.update(atom.end_line.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::index::{RepoAtom, RepoIndex};

    fn fake_index() -> RepoIndex {
        RepoIndex {
            root: r"C:\repo\huge".to_string(),
            updated_at: Utc::now(),
            atoms: vec![
                RepoAtom {
                    id: "qva_auth".to_string(),
                    path: "src/auth.rs".to_string(),
                    start_line: 1,
                    end_line: 80,
                    hash: "auth_hash".to_string(),
                    token_estimate: 5_000_000,
                    symbols: vec!["login".to_string()],
                    signal_mask: 0,
                    vector: vec![1, 2, 3],
                    text: "SECRET_AUTH_BODY_SHOULD_NOT_BE_IN_POINTER".to_string(),
                },
                RepoAtom {
                    id: "qva_billing".to_string(),
                    path: "src/billing.rs".to_string(),
                    start_line: 1,
                    end_line: 80,
                    hash: "billing_hash".to_string(),
                    token_estimate: 7_000_000,
                    symbols: vec!["invoice".to_string()],
                    signal_mask: 0,
                    vector: vec![4, 5, 6],
                    text: "SECRET_BILLING_BODY_SHOULD_NOT_BE_IN_POINTER".to_string(),
                },
            ],
        }
    }

    #[test]
    fn session_pointer_replaces_bulk_context_with_tiny_handle() {
        let pointer = super::build_session_pointer(&fake_index());

        assert!(pointer.handle.starts_with("qorx://s/"));
        assert_eq!(pointer.indexed_tokens, 12_000_000);
        assert!(pointer.visible_tokens < 80);
        assert!(pointer.context_reduction_x > 100_000.0);
        assert!(!pointer.prompt_block.contains("SECRET_AUTH_BODY"));
        assert!(!pointer.prompt_block.contains("SECRET_BILLING_BODY"));
        assert!(pointer.prompt_block.contains(&pointer.handle));
    }

    #[test]
    fn session_pointer_is_stable_for_same_index_state() {
        let index = fake_index();

        let first = super::build_session_pointer(&index);
        let second = super::build_session_pointer(&index);

        assert_eq!(first.handle, second.handle);
        assert_eq!(first.root_fingerprint, second.root_fingerprint);
    }

    #[test]
    fn session_pointer_boundary_is_not_fake_literal_compression() {
        let pointer = super::build_session_pointer(&fake_index());

        assert!(pointer.boundary.contains("local resolver handles"));
        assert!(pointer.boundary.contains("Qorx proxy"));
        assert!(pointer.prompt_block.contains("local_idx stays local"));
        assert!(pointer.prompt_block.contains("resolve with Qorx"));
        assert!(!pointer.prompt_block.contains(&["no", "mcp"].join("_")));
        assert!(!pointer
            .prompt_block
            .contains(&["no", "bulk_repo"].join("_")));
    }

    #[test]
    fn session_pointer_ends_with_compact_b2c_proof_tail() {
        let pointer = super::build_session_pointer(&fake_index());
        let proof_tail = pointer
            .prompt_block
            .lines()
            .last()
            .expect("session prompt should have a final proof line");

        assert!(proof_tail.starts_with("proof at="));
        assert!(proof_tail.contains("ctx=12000000t"));
        assert!(proof_tail.contains(&format!("vis={}t", pointer.visible_tokens)));
        assert!(proof_tail.contains(&format!("omitted={}t", pointer.omitted_tokens)));
        assert!(proof_tail.contains("qshf="));
        assert!(!proof_tail.contains("redshift="));
        assert!(proof_tail.contains("est=char4"));
        assert!(proof_tail.contains("b2c=accounting"));
    }

    #[test]
    fn session_pointer_runs_core_cost_stack_by_default() {
        let pointer = super::build_session_pointer(&fake_index());

        assert!(pointer.prompt_block.contains("qosm=core"));
        assert!(pointer.prompt_block.contains("qshf=core_b2c"));
        assert!(!pointer.prompt_block.contains("redshift=core_b2c"));
        assert!(pointer.prompt_block.contains("resolve with Qorx"));
    }
}
