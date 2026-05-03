use serde::{Deserialize, Serialize};

use crate::compression::estimate_tokens;

pub const DYNAMIC_MARKER: &str = "--- QORX_DYNAMIC ---";
const PROVIDER_CACHE_FLOOR_TOKENS: u64 = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePlan {
    pub schema: String,
    pub marker: String,
    pub prompt_tokens: u64,
    pub stable_prefix_tokens: u64,
    pub dynamic_tail_tokens: u64,
    pub estimated_cacheable_tokens: u64,
    pub provider_cache_floor_tokens: u64,
    pub provider_cache_floor_met: bool,
    pub can_cache_prefix: bool,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
    pub boundary: String,
}

pub fn plan_prompt(prompt: &str) -> CachePlan {
    let (stable_prefix, dynamic_tail, marker) = split_prompt(prompt);
    let prompt_tokens = estimate_tokens(prompt);
    let stable_prefix_tokens = estimate_tokens(stable_prefix);
    let dynamic_tail_tokens = estimate_tokens(dynamic_tail);
    let estimated_cacheable_tokens = stable_prefix_tokens;
    let provider_cache_floor_met = estimated_cacheable_tokens >= PROVIDER_CACHE_FLOOR_TOKENS;
    let can_cache_prefix = !stable_prefix.trim().is_empty() && !dynamic_tail.trim().is_empty();
    let mut recommendations = Vec::new();
    if marker == DYNAMIC_MARKER {
        recommendations.push(format!(
            "keep stable prefix first: marker already present; keep the {stable_prefix_tokens} estimated-token prefix stable across repeated calls"
        ));
    } else if can_cache_prefix {
        recommendations.push(format!(
            "keep stable prefix first: insert `{DYNAMIC_MARKER}` after the current {stable_prefix_tokens} estimated-token stable prefix"
        ));
    } else {
        recommendations.push(format!(
            "keep stable prefix first: add a reusable stable prefix before `{DYNAMIC_MARKER}`; current prompt has no separate dynamic tail"
        ));
    }
    recommendations.push(format!(
        "put the {dynamic_tail_tokens} estimated-token volatile tail after the marker: user turn, tool output, timestamps, and diffs"
    ));
    recommendations.push(
        "route repeated non-streaming provider calls through Qorx when you want local replay-cache telemetry"
            .to_string(),
    );
    if provider_cache_floor_met {
        recommendations
            .push("prefix is large enough for common provider prompt-cache floors".to_string());
    }
    let mut warnings = Vec::new();
    if !provider_cache_floor_met {
        warnings.push(format!(
            "stable prefix is below the common {PROVIDER_CACHE_FLOOR_TOKENS}-token provider cache floor; Qorx still benefits by reducing context before upstream calls"
        ));
    }
    if !can_cache_prefix {
        warnings.push("prompt has no clear stable/dynamic split".to_string());
    }

    CachePlan {
        schema: "qorx.cache-plan.v1".to_string(),
        marker: marker.to_string(),
        prompt_tokens,
        stable_prefix_tokens,
        dynamic_tail_tokens,
        estimated_cacheable_tokens,
        provider_cache_floor_tokens: PROVIDER_CACHE_FLOOR_TOKENS,
        provider_cache_floor_met,
        can_cache_prefix,
        recommendations,
        warnings,
        boundary: "Cache planning is deterministic prompt-layout advice plus token estimates. It does not create a provider cache entry by itself; actual cache hits depend on upstream provider rules, repeated prefixes, model, account, streaming mode, and response metadata.".to_string(),
    }
}

fn split_prompt(prompt: &str) -> (&str, &str, &str) {
    if let Some((stable, dynamic)) = prompt.split_once(DYNAMIC_MARKER) {
        return (stable.trim(), dynamic.trim(), DYNAMIC_MARKER);
    }
    if let Some((stable, dynamic)) = prompt.rsplit_once("\n\n") {
        return (stable.trim(), dynamic.trim(), "<last blank line>");
    }
    if let Some((stable, dynamic)) = prompt.rsplit_once('\n') {
        return (stable.trim(), dynamic.trim(), "<last line>");
    }
    (prompt.trim(), "", "<none>")
}

#[cfg(test)]
mod tests {
    #[test]
    fn dynamic_marker_splits_prompt() {
        let plan = super::plan_prompt("stable\n--- QORX_DYNAMIC ---\ndynamic");
        assert_eq!(plan.marker, super::DYNAMIC_MARKER);
        assert!(plan.can_cache_prefix);
    }
}
