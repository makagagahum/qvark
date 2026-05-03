use std::{collections::BTreeMap, path::Path};

use anyhow::Result;
use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Response, StatusCode},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const MAX_ENTRIES: usize = 512;
const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExactResponseCache {
    pub entries: BTreeMap<String, CachedResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub created_at: DateTime<Utc>,
    pub last_hit_at: Option<DateTime<Utc>>,
    pub hits: u64,
    pub status: u16,
    pub content_type: Option<String>,
    pub body_base64: String,
}

impl ExactResponseCache {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let legacy = path.with_extension("json");
        crate::proto_store::load_or_default(path, &[legacy.as_path()])
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        crate::proto_store::save(path.as_ref(), self)
    }

    pub fn get(&mut self, key: &str) -> Option<CachedResponse> {
        let entry = self.entries.get_mut(key)?;
        entry.hits += 1;
        entry.last_hit_at = Some(Utc::now());
        Some(entry.clone())
    }

    pub fn insert(
        &mut self,
        key: String,
        status: StatusCode,
        content_type: Option<String>,
        body: &[u8],
    ) {
        if body.len() > MAX_BODY_BYTES {
            return;
        }
        self.entries.insert(
            key,
            CachedResponse {
                created_at: Utc::now(),
                last_hit_at: None,
                hits: 0,
                status: status.as_u16(),
                content_type,
                body_base64: STANDARD.encode(body),
            },
        );
        self.prune();
    }

    fn prune(&mut self) {
        while self.entries.len() > MAX_ENTRIES {
            let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, value)| value.last_hit_at.unwrap_or(value.created_at))
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.entries.remove(&oldest_key);
        }
    }
}

pub fn exact_key(provider: &str, method: &Method, path: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(provider.as_bytes());
    hasher.update(method.as_str().as_bytes());
    hasher.update(path.as_bytes());
    hasher.update(body);
    format!("qvc_{:x}", hasher.finalize())
}

pub fn request_key(provider: &str, method: &Method, path: &str, body: &[u8]) -> Option<String> {
    is_cacheable_request(method, body)
        .then(|| exact_key(provider, method, path, &canonical_body(body)))
}

pub fn is_cacheable_request(method: &Method, body: &[u8]) -> bool {
    if method != Method::POST {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return false;
    };
    !contains_stream_true(&value)
}

fn canonical_body(body: &[u8]) -> Vec<u8> {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| serde_json::to_vec(&value).ok())
        .unwrap_or_else(|| body.to_vec())
}

pub fn response_from_cached(entry: CachedResponse) -> Response<Body> {
    let status = StatusCode::from_u16(entry.status).unwrap_or(StatusCode::OK);
    let bytes = STANDARD.decode(entry.body_base64).unwrap_or_default();
    let mut builder = Response::builder()
        .status(status)
        .header("x-qorx-cache", "hit");
    if let Some(content_type) = entry.content_type {
        builder = builder.header(CONTENT_TYPE, content_type);
    }
    builder.body(Body::from(bytes)).unwrap()
}

fn contains_stream_true(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            key.eq_ignore_ascii_case("stream") && value.as_bool().unwrap_or(false)
                || contains_stream_true(value)
        }),
        Value::Array(items) => items.iter().any(contains_stream_true),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use axum::http::Method;

    use super::*;

    #[test]
    fn streaming_requests_are_not_cacheable() {
        assert!(!is_cacheable_request(&Method::POST, br#"{"stream":true}"#));
        assert!(is_cacheable_request(&Method::POST, br#"{"stream":false}"#));
    }

    #[test]
    fn exact_key_changes_with_body() {
        let a = exact_key("openai", &Method::POST, "v1/chat", br#"{"a":1}"#);
        let b = exact_key("openai", &Method::POST, "v1/chat", br#"{"a":2}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn request_key_canonicalizes_json_whitespace_and_key_order() {
        let a = request_key(
            "openai",
            &Method::POST,
            "v1/chat",
            br#"{"stream":false,"model":"x","messages":[{"role":"user","content":"cache me"}]}"#,
        )
        .expect("cacheable request");
        let b = request_key(
            "openai",
            &Method::POST,
            "v1/chat",
            br#"{
              "messages": [{"content": "cache me", "role": "user"}],
              "model": "x",
              "stream": false
            }"#,
        )
        .expect("cacheable request");
        let c = request_key(
            "openai",
            &Method::POST,
            "v1/chat",
            br#"{"stream":false,"model":"x","messages":[{"role":"user","content":"different"}]}"#,
        )
        .expect("cacheable request");

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
