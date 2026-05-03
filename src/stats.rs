use std::{env, fs, path::Path, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const DEFAULT_INPUT_USD_PER_MILLION: f64 = 2.50;
const DEFAULT_CACHED_INPUT_USD_PER_MILLION: f64 = 0.25;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub requests: u64,
    pub raw_prompt_tokens: u64,
    pub compressed_prompt_tokens: u64,
    pub saved_prompt_tokens: u64,
    pub upstream_errors: u64,
    #[serde(alias = "atoms_created")]
    pub quarks_created: u64,
    pub cache_lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_saved_prompt_tokens: u64,
    pub provider_cached_prompt_tokens: u64,
    pub provider_cache_write_tokens: u64,
    pub context_pack_requests: u64,
    pub context_indexed_tokens: u64,
    pub context_sent_tokens: u64,
    pub context_omitted_tokens: u64,
    pub last_provider: Option<String>,
}

impl Default for Stats {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            started_at: now,
            updated_at: now,
            requests: 0,
            raw_prompt_tokens: 0,
            compressed_prompt_tokens: 0,
            saved_prompt_tokens: 0,
            upstream_errors: 0,
            quarks_created: 0,
            cache_lookups: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_saved_prompt_tokens: 0,
            provider_cached_prompt_tokens: 0,
            provider_cache_write_tokens: 0,
            context_pack_requests: 0,
            context_indexed_tokens: 0,
            context_sent_tokens: 0,
            context_omitted_tokens: 0,
            last_provider: None,
        }
    }
}

impl Stats {
    pub fn savings_percent(&self) -> f64 {
        if self.raw_prompt_tokens == 0 {
            0.0
        } else {
            (self.saved_prompt_tokens as f64 / self.raw_prompt_tokens as f64) * 100.0
        }
    }

    pub fn atomic_ratio(&self) -> f64 {
        if self.compressed_prompt_tokens == 0 {
            1.0
        } else {
            self.raw_prompt_tokens.max(1) as f64 / self.compressed_prompt_tokens.max(1) as f64
        }
    }

    pub fn quark_ratio(&self) -> f64 {
        self.atomic_ratio()
    }

    pub fn context_reduction_x(&self) -> f64 {
        if self.context_sent_tokens == 0 {
            1.0
        } else {
            self.context_indexed_tokens.max(1) as f64 / self.context_sent_tokens.max(1) as f64
        }
    }

    pub fn pricing(&self) -> Pricing {
        Pricing::from_env()
    }

    pub fn context_usd_saved(&self) -> f64 {
        self.pricing().input_usd(self.context_omitted_tokens)
    }

    pub fn proxy_usd_saved(&self) -> f64 {
        self.pricing().input_usd(self.saved_prompt_tokens)
    }

    pub fn provider_cache_usd_saved(&self) -> f64 {
        self.pricing()
            .cached_discount_usd(self.provider_cached_prompt_tokens)
    }

    pub fn total_estimated_usd_saved(&self) -> f64 {
        self.context_usd_saved() + self.proxy_usd_saved() + self.provider_cache_usd_saved()
    }

    pub fn cache_hit_rate_percent(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.requests as f64) * 100.0
        }
    }

    pub fn cache_lookup_hit_rate_percent(&self) -> f64 {
        if self.cache_lookups == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.cache_lookups as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    pub input_usd_per_million_tokens: f64,
    pub cached_input_usd_per_million_tokens: f64,
    pub source: String,
    pub assumption: String,
}

impl Pricing {
    pub fn from_env() -> Self {
        let input_env = env::var("QORX_USD_PER_M_INPUT_TOKENS").ok();
        let cached_env = env::var("QORX_USD_PER_M_CACHED_INPUT_TOKENS").ok();
        let input = input_env
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| *value >= 0.0)
            .unwrap_or(DEFAULT_INPUT_USD_PER_MILLION);
        let cached = cached_env
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| *value >= 0.0)
            .unwrap_or(DEFAULT_CACHED_INPUT_USD_PER_MILLION);
        let source = if input_env.is_some() || cached_env.is_some() {
            "env_override".to_string()
        } else {
            "default_example_rates_2026_04_28".to_string()
        };

        Self {
            input_usd_per_million_tokens: input,
            cached_input_usd_per_million_tokens: cached,
            source,
            assumption: "Dollar savings are estimates from configured input-token prices; set QORX_USD_PER_M_INPUT_TOKENS and QORX_USD_PER_M_CACHED_INPUT_TOKENS for the actual model/account.".to_string(),
        }
    }

    pub fn input_usd(&self, tokens: u64) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.input_usd_per_million_tokens
    }

    pub fn cached_discount_usd(&self, tokens: u64) -> f64 {
        let discount =
            (self.input_usd_per_million_tokens - self.cached_input_usd_per_million_tokens).max(0.0);
        (tokens as f64 / 1_000_000.0) * discount
    }
}

pub fn record_context_pack(
    path: impl AsRef<Path>,
    indexed_tokens: u64,
    sent_tokens: u64,
) -> Result<()> {
    let path = path.as_ref();
    let legacy = path.with_extension("json");
    let mut stats: Stats = crate::proto_store::load_or_default(path, &[legacy.as_path()])?;

    stats.updated_at = Utc::now();
    stats.context_pack_requests += 1;
    stats.context_indexed_tokens += indexed_tokens;
    stats.context_sent_tokens += sent_tokens;
    stats.context_omitted_tokens += indexed_tokens.saturating_sub(sent_tokens);
    crate::proto_store::save(path, &stats)?;
    Ok(())
}

pub fn reset(path: impl AsRef<Path>) -> Result<Stats> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let stats = Stats::default();
    crate::proto_store::save(path, &stats)?;
    Ok(stats)
}

#[derive(Debug, Clone)]
pub struct RequestStats<'a> {
    pub provider: &'a str,
    pub raw_prompt_tokens: u64,
    pub compressed_prompt_tokens: u64,
    pub quarks_created: u64,
    pub upstream_error: bool,
    pub cache_lookup: bool,
    pub cache_hit: bool,
    pub provider_cached_prompt_tokens: u64,
    pub provider_cache_write_tokens: u64,
}

#[derive(Clone)]
pub struct StatsStore {
    path: Arc<std::path::PathBuf>,
    inner: Arc<Mutex<Stats>>,
}

impl StatsStore {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let legacy = path.with_extension("json");
        let stats = crate::proto_store::load_or_default(&path, &[legacy.as_path()])?;

        Ok(Self {
            path: Arc::new(path),
            inner: Arc::new(Mutex::new(stats)),
        })
    }

    pub async fn snapshot(&self) -> Stats {
        self.inner.lock().await.clone()
    }

    pub async fn record_request(&self, request: RequestStats<'_>) -> Result<()> {
        let mut stats = self.inner.lock().await;
        stats.updated_at = Utc::now();
        stats.requests += 1;
        stats.raw_prompt_tokens += request.raw_prompt_tokens;
        stats.compressed_prompt_tokens += request.compressed_prompt_tokens;
        stats.saved_prompt_tokens += request
            .raw_prompt_tokens
            .saturating_sub(request.compressed_prompt_tokens);
        stats.quarks_created += request.quarks_created;
        if request.cache_lookup {
            stats.cache_lookups += 1;
        }
        stats.provider_cached_prompt_tokens += request.provider_cached_prompt_tokens;
        stats.provider_cache_write_tokens += request.provider_cache_write_tokens;
        stats.last_provider = Some(request.provider.to_string());
        if request.cache_hit {
            stats.cache_hits += 1;
            stats.cache_saved_prompt_tokens += request.raw_prompt_tokens;
        } else if request.cache_lookup {
            stats.cache_misses += 1;
        }
        if request.upstream_error {
            stats.upstream_errors += 1;
        }
        crate::proto_store::save(&self.path, &*stats)?;
        Ok(())
    }

    pub async fn reset(&self) -> Result<Stats> {
        let mut stats = self.inner.lock().await;
        *stats = Stats::default();
        crate::proto_store::save(&self.path, &*stats)?;
        Ok(stats.clone())
    }
}
