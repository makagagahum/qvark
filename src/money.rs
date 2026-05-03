use serde::{Deserialize, Serialize};

use crate::stats::{Pricing, Stats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyProof {
    pub schema: String,
    pub qorx_version: String,
    pub production_gate_passed: bool,
    pub verdict: String,
    pub reasons: Vec<String>,
    pub observed: MoneyObserved,
    pub pricing: Pricing,
    pub claim_check: Option<ClaimCheck>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyObserved {
    pub context_pack_requests: u64,
    pub routed_provider_requests: u64,
    pub upstream_errors: u64,
    pub context_indexed_tokens: u64,
    pub context_sent_tokens: u64,
    pub context_omitted_tokens: u64,
    pub routed_raw_prompt_tokens: u64,
    pub routed_compressed_prompt_tokens: u64,
    pub routed_saved_prompt_tokens: u64,
    pub response_cache_hits: u64,
    pub response_cache_saved_prompt_tokens: u64,
    pub provider_cached_prompt_tokens: u64,
    pub provider_cache_write_tokens: u64,
    pub estimated_repo_context_usd_saved: f64,
    pub estimated_routed_proxy_usd_saved: f64,
    pub estimated_provider_cache_usd_saved: f64,
    pub estimated_total_usd_saved: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimCheck {
    pub claimed_usd: f64,
    pub observed_usd: f64,
    pub allowed: bool,
    pub rejection_reason: Option<String>,
}

pub fn build_money_proof(stats: &Stats, claimed_usd: Option<f64>) -> MoneyProof {
    let pricing = stats.pricing();
    let observed = MoneyObserved {
        context_pack_requests: stats.context_pack_requests,
        routed_provider_requests: stats.requests,
        upstream_errors: stats.upstream_errors,
        context_indexed_tokens: stats.context_indexed_tokens,
        context_sent_tokens: stats.context_sent_tokens,
        context_omitted_tokens: stats.context_omitted_tokens,
        routed_raw_prompt_tokens: stats.raw_prompt_tokens,
        routed_compressed_prompt_tokens: stats.compressed_prompt_tokens,
        routed_saved_prompt_tokens: stats.saved_prompt_tokens,
        response_cache_hits: stats.cache_hits,
        response_cache_saved_prompt_tokens: stats.cache_saved_prompt_tokens,
        provider_cached_prompt_tokens: stats.provider_cached_prompt_tokens,
        provider_cache_write_tokens: stats.provider_cache_write_tokens,
        estimated_repo_context_usd_saved: stats.context_usd_saved(),
        estimated_routed_proxy_usd_saved: stats.proxy_usd_saved(),
        estimated_provider_cache_usd_saved: stats.provider_cache_usd_saved(),
        estimated_total_usd_saved: stats.total_estimated_usd_saved(),
    };

    let mut reasons = Vec::new();
    if observed.context_pack_requests == 0 {
        reasons
            .push("no local pack/impact savings have been recorded since stats reset".to_string());
    }
    if observed.routed_provider_requests == 0 {
        reasons.push("no provider requests have routed through Qorx since stats reset".to_string());
    }
    if observed.routed_provider_requests > 0
        && observed.routed_provider_requests == observed.upstream_errors
    {
        reasons.push("all routed provider requests failed upstream".to_string());
    }
    if observed.routed_provider_requests > 0
        && observed.routed_saved_prompt_tokens == 0
        && observed.response_cache_hits == 0
        && observed.provider_cached_prompt_tokens == 0
    {
        reasons.push(
            "routed provider traffic exists but no billable prompt savings are visible yet"
                .to_string(),
        );
    }

    let billable_provider_savings_visible = observed.routed_provider_requests > 0
        && observed.routed_provider_requests > observed.upstream_errors
        && (observed.routed_saved_prompt_tokens > 0
            || observed.response_cache_hits > 0
            || observed.provider_cached_prompt_tokens > 0);
    let production_gate_passed =
        observed.context_pack_requests > 0 && billable_provider_savings_visible;

    let claim_check = claimed_usd.map(|claimed| {
        let observed_usd = observed.estimated_total_usd_saved;
        let allowed = production_gate_passed && observed_usd >= claimed;
        ClaimCheck {
            claimed_usd: claimed,
            observed_usd,
            allowed,
            rejection_reason: (!allowed).then(|| {
                if !production_gate_passed {
                    "production money gate has not passed with routed provider savings evidence"
                        .to_string()
                } else {
                    "observed estimated savings are below the claimed amount".to_string()
                }
            }),
        }
    });

    let verdict = if production_gate_passed {
        "production_money_gate_passed".to_string()
    } else {
        "not_production_billable_yet".to_string()
    };

    MoneyProof {
        schema: "qorx.money-proof.v1".to_string(),
        qorx_version: env!("CARGO_PKG_VERSION").to_string(),
        production_gate_passed,
        verdict,
        reasons,
        observed,
        pricing,
        claim_check,
        boundary: "Dollar savings are estimates from local token counters multiplied by the pricing object in this report. Billable provider savings require provider traffic routed through Qorx plus recorded proxy/cache/provider-cache counters. Streaming provider cache metadata may be unavailable unless the provider exposes it before the stream body. Billion-dollar or production claims are rejected unless observed counters and configured rates support them.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::stats::Stats;

    #[test]
    fn rejects_billion_dollar_claim_without_routed_provider_evidence() {
        let stats = Stats {
            context_pack_requests: 1,
            context_indexed_tokens: 80_000,
            context_sent_tokens: 500,
            context_omitted_tokens: 79_500,
            ..Stats::default()
        };

        let proof = super::build_money_proof(&stats, Some(5_000_000_000.0));

        assert!(!proof.production_gate_passed);
        assert_eq!(proof.verdict, "not_production_billable_yet");
        assert!(proof
            .reasons
            .iter()
            .any(|reason| reason.contains("no provider requests")));
        assert!(!proof.claim_check.unwrap().allowed);
    }

    #[test]
    fn passes_gate_when_context_and_routed_savings_are_observed() {
        let stats = Stats {
            requests: 2,
            raw_prompt_tokens: 20_000,
            compressed_prompt_tokens: 2_000,
            saved_prompt_tokens: 18_000,
            context_pack_requests: 1,
            context_indexed_tokens: 80_000,
            context_sent_tokens: 500,
            context_omitted_tokens: 79_500,
            ..Stats::default()
        };

        let proof = super::build_money_proof(&stats, Some(0.01));

        assert!(proof.production_gate_passed);
        assert_eq!(proof.verdict, "production_money_gate_passed");
        assert!(proof.claim_check.unwrap().allowed);
    }
}
