use serde::{Deserialize, Serialize};

use crate::{
    index::RepoIndex,
    truth::{self, StrictEvidence},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeReport {
    pub schema: String,
    pub answer: String,
    pub query: Option<String>,
    pub local_only: bool,
    pub provider_calls: u64,
    pub supported_claims: usize,
    pub partial_claims: usize,
    pub unsupported_claims: usize,
    pub claims: Vec<ClaimJudgement>,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimJudgement {
    pub claim: String,
    pub verdict: String,
    pub evidence: Vec<StrictEvidence>,
    pub missing_terms: Vec<String>,
    pub used_tokens: u64,
}

pub fn judge_answer(index: &RepoIndex, answer: &str, query: Option<&str>) -> JudgeReport {
    let claims = split_claims(answer)
        .into_iter()
        .map(|claim| {
            let probe = query
                .filter(|query| !query.trim().is_empty())
                .map(|query| format!("{query} {claim}"))
                .unwrap_or_else(|| claim.clone());
            let strict = truth::strict_answer(index, &probe, 2);
            let verdict = match strict.coverage.as_str() {
                "supported" => "supported",
                "partial" => "partial",
                _ => "unsupported",
            }
            .to_string();
            ClaimJudgement {
                claim,
                verdict,
                evidence: strict.evidence,
                missing_terms: strict.missing_terms,
                used_tokens: strict.used_tokens,
            }
        })
        .collect::<Vec<_>>();
    let supported_claims = claims
        .iter()
        .filter(|claim| claim.verdict == "supported")
        .count();
    let partial_claims = claims
        .iter()
        .filter(|claim| claim.verdict == "partial")
        .count();
    let unsupported_claims = claims
        .iter()
        .filter(|claim| claim.verdict == "unsupported")
        .count();

    JudgeReport {
        schema: "qorx.judge.v1".to_string(),
        answer: answer.to_string(),
        query: query.map(ToOwned::to_owned),
        local_only: true,
        provider_calls: 0,
        supported_claims,
        partial_claims,
        unsupported_claims,
        claims,
        boundary: "Judge checks answer claims against indexed local evidence only. It can prove support, partial support, or refusal for local context; external world truth is outside this local evidence certificate.".to_string(),
    }
}

fn split_claims(answer: &str) -> Vec<String> {
    let mut claims = Vec::new();
    let mut current = String::new();
    for ch in answer.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            push_claim(&mut claims, &mut current);
        }
    }
    push_claim(&mut claims, &mut current);
    claims
}

fn push_claim(claims: &mut Vec<String>, current: &mut String) {
    let claim = current
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_string();
    current.clear();
    if claim.split_whitespace().count() >= 3 {
        claims.push(claim);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn split_claims_ignores_tiny_fragments() {
        let claims = super::split_claims("yes. production gate requires proof.");
        assert_eq!(claims, ["production gate requires proof"]);
    }
}
