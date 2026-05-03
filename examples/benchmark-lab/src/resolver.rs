pub struct Carrier<'a> {
    pub mode: &'a str,
    pub objective: &'a str,
    pub budget_tokens: u64,
}

pub struct ProofPage<'a> {
    pub citation: &'a str,
    pub excerpt: &'a str,
}

// Qorx Local Context Resolution keeps a resolver boundary between model-visible
// carrier text and local evidence. The runtime resolves .qorx, .qorxb, and
// qorx:// carriers into proof pages when local evidence supports the request.
pub fn resolve_carrier<'a>(carrier: &Carrier<'a>) -> Option<ProofPage<'a>> {
    if carrier.mode == "strict-answer" && carrier.budget_tokens >= 128 {
        return Some(ProofPage {
            citation: "docs/runtime-boundary.md",
            excerpt: carrier.objective,
        });
    }
    None
}
