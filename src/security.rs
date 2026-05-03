use std::{env, fs, path::Path, process::Command};

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use ed25519_dalek::{
    Signature as Ed25519Signature, Signer as Ed25519Signer, SigningKey as Ed25519SigningKey,
    Verifier as Ed25519Verifier, VerifyingKey as Ed25519VerifyingKey,
};
use ml_dsa::{
    signature::{Keypair, SignatureEncoding, Signer, Verifier},
    KeyGen, MlDsa65,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    aim::{inspect_default, AimReport},
    config::AppPaths,
    index::load_index,
    session::build_session_pointer,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceManifest {
    pub schema: String,
    pub created_at: String,
    pub qorx_version: String,
    pub standards: ProvenanceStandards,
    pub subject: ProvenanceSubject,
    pub signatures: ProvenanceSignatures,
    pub verified: bool,
    pub boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceStandards {
    pub classical: String,
    pub post_quantum: String,
    pub c2pa: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceSubject {
    pub session_handle: String,
    pub session_root_fingerprint: String,
    pub indexed_tokens: u64,
    pub q_drive_target: String,
    pub aim: AimReport,
    pub canonical_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceSignatures {
    pub ed25519_public_key_b64: String,
    pub ed25519_signature_b64: String,
    pub ml_dsa_parameter_set: String,
    pub ml_dsa_public_key_b64: String,
    pub ml_dsa_signature_b64: String,
    pub ml_dsa_public_key_sha256: String,
    pub ml_dsa_signature_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SigningSeeds {
    ed25519_seed_b64: String,
    ml_dsa65_seed_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsignedProvenance {
    schema: String,
    created_at: String,
    qorx_version: String,
    standards: ProvenanceStandards,
    subject: ProvenanceSubject,
}

pub fn attest(paths: &AppPaths) -> Result<ProvenanceManifest> {
    let unsigned = build_unsigned(paths)?;
    let canonical = serde_json::to_vec(&unsigned)?;
    let seeds = load_or_create_seeds(&paths.security_keys_file)?;
    let signatures = sign_canonical(&seeds, &canonical)?;
    let verified = verify_canonical(&signatures, &canonical)?;
    let manifest = ProvenanceManifest {
        schema: unsigned.schema,
        created_at: unsigned.created_at,
        qorx_version: unsigned.qorx_version,
        standards: unsigned.standards,
        subject: unsigned.subject,
        signatures,
        verified,
        boundary: "Hybrid provenance signs Qorx/AIM state with Ed25519 and ML-DSA-65. Private seed storage is local filesystem ACL-protected on creation where the OS allows it, but it is not passphrase-encrypted or stored in a hardware/OS key vault. This is C2PA-style provenance metadata, not a full embedded C2PA asset manifest or FIPS validation certificate.".to_string(),
    };
    crate::proto_store::save(&paths.provenance_file, &manifest)?;
    Ok(manifest)
}

pub fn verify_saved(paths: &AppPaths) -> Result<ProvenanceManifest> {
    let legacy = paths.provenance_file.with_extension("json");
    let mut manifest: ProvenanceManifest =
        crate::proto_store::load_required(&paths.provenance_file, &[legacy.as_path()])?;
    let unsigned = UnsignedProvenance {
        schema: manifest.schema.clone(),
        created_at: manifest.created_at.clone(),
        qorx_version: manifest.qorx_version.clone(),
        standards: manifest.standards.clone(),
        subject: manifest.subject.clone(),
    };
    let canonical = serde_json::to_vec(&unsigned)?;
    manifest.verified = verify_canonical(&manifest.signatures, &canonical)?;
    Ok(manifest)
}

fn build_unsigned(paths: &AppPaths) -> Result<UnsignedProvenance> {
    let index = load_index(&paths.index_file)?;
    let session = build_session_pointer(&index);
    let aim = inspect_default()?;
    let subject = ProvenanceSubject {
        session_handle: session.handle,
        session_root_fingerprint: session.root_fingerprint,
        indexed_tokens: session.indexed_tokens,
        q_drive_target: paths.data_dir.display().to_string(),
        aim,
        canonical_sha256: String::new(),
    };
    let standards = ProvenanceStandards {
        classical: "Ed25519 EdDSA, RFC 8032".to_string(),
        post_quantum: "ML-DSA-65, NIST FIPS 204".to_string(),
        c2pa: "C2PA-style signed provenance claim; full C2PA embedding is adapter scope"
            .to_string(),
    };
    let mut unsigned = UnsignedProvenance {
        schema: "qorx.hybrid-provenance.v1".to_string(),
        created_at: Utc::now().to_rfc3339(),
        qorx_version: env!("CARGO_PKG_VERSION").to_string(),
        standards,
        subject,
    };
    let canonical_without_hash = serde_json::to_vec(&unsigned)?;
    unsigned.subject.canonical_sha256 = hex_sha256(&canonical_without_hash);
    Ok(unsigned)
}

fn load_or_create_seeds(path: &Path) -> Result<SigningSeeds> {
    let legacy = path.with_extension("json");
    if path.exists() || legacy.exists() {
        return crate::proto_store::load_required(path, &[legacy.as_path()]);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut ed_seed = [0u8; 32];
    let mut ml_seed = [0u8; 32];
    getrandom::fill(&mut ed_seed).map_err(|err| anyhow!("getrandom failed: {err:?}"))?;
    getrandom::fill(&mut ml_seed).map_err(|err| anyhow!("getrandom failed: {err:?}"))?;
    let seeds = SigningSeeds {
        ed25519_seed_b64: B64.encode(ed_seed),
        ml_dsa65_seed_b64: B64.encode(ml_seed),
    };
    crate::proto_store::save(path, &seeds)?;
    harden_private_seed_file(path);
    Ok(seeds)
}

fn harden_private_seed_file(path: &Path) {
    #[cfg(windows)]
    {
        let Some(user) = env::var("USERNAME").ok().filter(|value| !value.is_empty()) else {
            return;
        };
        let _ = Command::new("icacls")
            .arg(path)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!("{user}:F"))
            .output();
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            let _ = fs::set_permissions(path, permissions);
        }
    }
}

fn sign_canonical(seeds: &SigningSeeds, canonical: &[u8]) -> Result<ProvenanceSignatures> {
    let ed_seed = decode_32(&seeds.ed25519_seed_b64)?;
    let ed_key = Ed25519SigningKey::from_bytes(&ed_seed);
    let ed_sig = ed_key.sign(canonical);

    let ml_seed = decode_32(&seeds.ml_dsa65_seed_b64)?;
    let ml_seed = ml_dsa::Seed::from(ml_seed);
    let ml_key = MlDsa65::from_seed(&ml_seed);
    let ml_sig = ml_key.sign(canonical);
    let ml_vk = ml_key.verifying_key();
    let ml_vk_bytes = ml_vk.encode();
    let ml_sig_bytes = ml_sig.to_bytes();

    Ok(ProvenanceSignatures {
        ed25519_public_key_b64: B64.encode(ed_key.verifying_key().to_bytes()),
        ed25519_signature_b64: B64.encode(ed_sig.to_bytes()),
        ml_dsa_parameter_set: "ML-DSA-65".to_string(),
        ml_dsa_public_key_b64: B64.encode(ml_vk_bytes.as_slice()),
        ml_dsa_signature_b64: B64.encode(ml_sig_bytes.as_slice()),
        ml_dsa_public_key_sha256: hex_sha256(ml_vk_bytes.as_slice()),
        ml_dsa_signature_sha256: hex_sha256(ml_sig_bytes.as_slice()),
    })
}

fn verify_canonical(signatures: &ProvenanceSignatures, canonical: &[u8]) -> Result<bool> {
    let ed_public = decode_32(&signatures.ed25519_public_key_b64)?;
    let ed_signature_bytes = B64.decode(&signatures.ed25519_signature_b64)?;
    let ed_signature_array: [u8; 64] = ed_signature_bytes
        .try_into()
        .map_err(|_| anyhow!("invalid Ed25519 signature length"))?;
    let ed_vk = Ed25519VerifyingKey::from_bytes(&ed_public)?;
    let ed_sig = Ed25519Signature::from_bytes(&ed_signature_array);
    ed_vk.verify(canonical, &ed_sig)?;

    let ml_public = B64.decode(&signatures.ml_dsa_public_key_b64)?;
    let ml_signature = B64.decode(&signatures.ml_dsa_signature_b64)?;
    let ml_vk_encoded = ml_dsa::EncodedVerifyingKey::<MlDsa65>::try_from(ml_public.as_slice())
        .map_err(|_| anyhow!("invalid ML-DSA public key length"))?;
    let ml_vk = ml_dsa::VerifyingKey::<MlDsa65>::decode(&ml_vk_encoded);
    let ml_sig = ml_dsa::Signature::<MlDsa65>::try_from(ml_signature.as_slice())
        .map_err(|_| anyhow!("invalid ML-DSA signature"))?;
    ml_vk.verify(canonical, &ml_sig)?;
    Ok(true)
}

fn decode_32(value: &str) -> Result<[u8; 32]> {
    let bytes = B64.decode(value)?;
    bytes
        .try_into()
        .map_err(|_| anyhow!("expected 32-byte seed or public key"))
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppPaths;
    use chrono::Utc;
    use std::fs;

    #[test]
    fn hybrid_signatures_verify_roundtrip() {
        let tmp = std::env::temp_dir().join(format!(
            "qorx-security-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(&tmp).unwrap();
        let paths = AppPaths {
            data_dir: tmp.clone(),
            portable: false,
            stats_file: tmp.join("stats.pb"),
            atom_file: tmp.join("quarks.pb"),
            index_file: tmp.join("repo_index.pb"),
            context_protobuf_file: tmp.join("qorx-context.pb"),
            response_cache_file: tmp.join("response_cache.pb"),
            integration_report_file: tmp.join("integrations.pb"),
            shim_dir: tmp.join("shims"),
            provenance_file: tmp.join("provenance.pb"),
            security_keys_file: tmp.join("security-keys.pb"),
        };
        crate::proto_store::save(
            &paths.index_file,
            &serde_json::json!({
                "root": "C:/repo",
                "updated_at": Utc::now(),
                "quarks": [{
                    "id": "qva_test",
                    "path": "src/lib.rs",
                    "start_line": 1,
                    "end_line": 2,
                    "hash": "abc",
                    "token_estimate": 10,
                    "symbols": ["demo"],
                    "signal_mask": 0,
                    "vector": [1,2],
                    "text": "fn demo() {}"
                }]
            }),
        )
        .unwrap();

        let manifest = attest(&paths).unwrap();
        assert!(manifest.verified);
        assert_eq!(manifest.signatures.ml_dsa_parameter_set, "ML-DSA-65");
        assert!(verify_saved(&paths).unwrap().verified);

        let _ = fs::remove_dir_all(tmp);
    }
}
