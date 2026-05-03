use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use prost::Message;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Number, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, PartialEq, Message)]
struct StateEnvelopePb {
    #[prost(string, tag = "1")]
    schema: String,
    #[prost(string, tag = "2")]
    created_at: String,
    #[prost(string, tag = "3")]
    payload_sha256: String,
    #[prost(message, optional, tag = "4")]
    payload: Option<StateValuePb>,
}

#[derive(Clone, PartialEq, Message)]
struct StateValuePb {
    #[prost(oneof = "state_value_pb::Kind", tags = "1, 2, 3, 4, 5, 6, 7, 8")]
    kind: Option<state_value_pb::Kind>,
}

mod state_value_pb {
    use super::{StateListPb, StateObjectPb};
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Kind {
        #[prost(bool, tag = "1")]
        Null(bool),
        #[prost(bool, tag = "2")]
        Bool(bool),
        #[prost(string, tag = "3")]
        String(String),
        #[prost(int64, tag = "4")]
        I64(i64),
        #[prost(uint64, tag = "5")]
        U64(u64),
        #[prost(double, tag = "6")]
        F64(f64),
        #[prost(message, tag = "7")]
        Object(StateObjectPb),
        #[prost(message, tag = "8")]
        List(StateListPb),
    }
}

#[derive(Clone, PartialEq, Message)]
struct StateObjectPb {
    #[prost(message, repeated, tag = "1")]
    fields: Vec<StateFieldPb>,
}

#[derive(Clone, PartialEq, Message)]
struct StateFieldPb {
    #[prost(string, tag = "1")]
    key: String,
    #[prost(message, optional, tag = "2")]
    value: Option<StateValuePb>,
}

#[derive(Clone, PartialEq, Message)]
struct StateListPb {
    #[prost(message, repeated, tag = "1")]
    values: Vec<StateValuePb>,
}

#[derive(Clone, PartialEq, Message)]
struct LegacyJsonEnvelopePb {
    #[prost(string, tag = "1")]
    schema: String,
    #[prost(string, tag = "2")]
    created_at: String,
    #[prost(string, tag = "3")]
    json_sha256: String,
    #[prost(bytes, tag = "4")]
    json: Vec<u8>,
}

pub fn load_or_default<T>(path: &Path, legacy_paths: &[&Path]) -> Result<T>
where
    T: DeserializeOwned + Default + Serialize,
{
    match load_required(path, legacy_paths) {
        Ok(value) => Ok(value),
        Err(_) => Ok(T::default()),
    }
}

pub fn load_required<T>(path: &Path, legacy_paths: &[&Path]) -> Result<T>
where
    T: DeserializeOwned + Serialize,
{
    if path.exists() {
        return match read_pb(path) {
            Ok(value) => Ok(value),
            Err(primary_error) => match read_legacy_json_envelope(path) {
                Ok(value) => {
                    save(path, &value)?;
                    Ok(value)
                }
                Err(_) => Err(primary_error),
            },
        };
    }

    for legacy_path in legacy_paths {
        if legacy_path.exists() {
            let value = read_json(legacy_path)?;
            save(path, &value)?;
            return Ok(value);
        }
    }

    Err(anyhow!("state file not found: {}", path.display()))
}

pub fn save<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let value = serde_json::to_value(value)?;
    let canonical = serde_json::to_vec(&value)?;
    let envelope = StateEnvelopePb {
        schema: "qorx.protobuf-state-envelope.v2".to_string(),
        created_at: Utc::now().to_rfc3339(),
        payload_sha256: hex_sha256(&canonical),
        payload: Some(json_to_pb(value)),
    };
    let mut encoded = Vec::new();
    envelope.encode(&mut encoded)?;
    fs::write(path, encoded)?;
    Ok(())
}

fn read_pb<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let bytes =
        fs::read(path).with_context(|| format!("could not read protobuf {}", path.display()))?;
    let envelope = StateEnvelopePb::decode(bytes.as_slice())
        .with_context(|| format!("could not decode protobuf {}", path.display()))?;
    let payload = envelope
        .payload
        .ok_or_else(|| anyhow!("protobuf payload missing for {}", path.display()))?;
    let value = pb_to_json(payload)?;
    let canonical = serde_json::to_vec(&value)?;
    let actual = hex_sha256(&canonical);
    if actual != envelope.payload_sha256 {
        return Err(anyhow!(
            "protobuf payload hash mismatch for {}",
            path.display()
        ));
    }
    Ok(serde_json::from_value(value)?)
}

fn read_legacy_json_envelope<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let bytes = fs::read(path)?;
    let envelope = LegacyJsonEnvelopePb::decode(bytes.as_slice())?;
    let actual = hex_sha256(&envelope.json);
    if actual != envelope.json_sha256 {
        return Err(anyhow!(
            "legacy protobuf JSON payload hash mismatch for {}",
            path.display()
        ));
    }
    Ok(serde_json::from_slice(&envelope.json)?)
}

fn read_json<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let text = fs::read_to_string(path)
        .with_context(|| format!("could not read legacy JSON {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn json_to_pb(value: Value) -> StateValuePb {
    use state_value_pb::Kind;

    let kind = match value {
        Value::Null => Kind::Null(true),
        Value::Bool(value) => Kind::Bool(value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Kind::I64(value)
            } else if let Some(value) = value.as_u64() {
                Kind::U64(value)
            } else {
                Kind::F64(value.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(value) => Kind::String(value),
        Value::Array(values) => Kind::List(StateListPb {
            values: values.into_iter().map(json_to_pb).collect(),
        }),
        Value::Object(fields) => Kind::Object(StateObjectPb {
            fields: fields
                .into_iter()
                .map(|(key, value)| StateFieldPb {
                    key,
                    value: Some(json_to_pb(value)),
                })
                .collect(),
        }),
    };

    StateValuePb { kind: Some(kind) }
}

fn pb_to_json(value: StateValuePb) -> Result<Value> {
    use state_value_pb::Kind;

    Ok(
        match value
            .kind
            .ok_or_else(|| anyhow!("protobuf state value missing"))?
        {
            Kind::Null(_) => Value::Null,
            Kind::Bool(value) => Value::Bool(value),
            Kind::String(value) => Value::String(value),
            Kind::I64(value) => Value::Number(Number::from(value)),
            Kind::U64(value) => Value::Number(Number::from(value)),
            Kind::F64(value) => Value::Number(
                Number::from_f64(value).ok_or_else(|| anyhow!("invalid protobuf f64 value"))?,
            ),
            Kind::List(list) => Value::Array(
                list.values
                    .into_iter()
                    .map(pb_to_json)
                    .collect::<Result<Vec<_>>>()?,
            ),
            Kind::Object(object) => {
                let mut fields = serde_json::Map::new();
                for field in object.fields {
                    let value = field
                        .value
                        .ok_or_else(|| anyhow!("protobuf object field missing value"))?;
                    fields.insert(field.key, pb_to_json(value)?);
                }
                Value::Object(fields)
            }
        },
    )
}
