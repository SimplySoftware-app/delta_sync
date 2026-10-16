//! Shared by the integration tests. Keys and certificates are made at run time.
#![allow(dead_code)]

pub mod mock_tsa;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use delta_sync_core::{CoreError, DevAuthority, DeviceSigner, IssuedCert, SignerAlg};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::pkcs8::{EncodePublicKey, LineEnding};
use rand_core::{OsRng, RngCore};
use time::OffsetDateTime;

/// A signer whose key lives in Rust, standing in for the Secure Enclave and
/// the Android Keystore.
pub struct P256Signer {
    key: SigningKey,
    chain: String,
}

impl P256Signer {
    /// A device under the Device ICA with a certificate valid now.
    pub fn device(authority: &DevAuthority) -> Arc<Self> {
        Self::from_cert(authority, |pem| {
            authority.issue(pem, "test-device").unwrap()
        })
    }

    /// A device whose certificate is valid only between the two instants.
    pub fn device_between(
        authority: &DevAuthority,
        not_before: OffsetDateTime,
        not_after: OffsetDateTime,
    ) -> Arc<Self> {
        Self::from_cert(authority, |pem| {
            authority
                .issue_device_between(pem, "test-device", not_before, not_after)
                .unwrap()
        })
    }

    /// The media processor, under the Service ICA.
    pub fn service(authority: &DevAuthority) -> Arc<Self> {
        Self::from_cert(authority, |pem| {
            authority.issue_service(pem, "test-service").unwrap()
        })
    }

    fn from_cert(authority: &DevAuthority, issue: impl FnOnce(&str) -> IssuedCert) -> Arc<Self> {
        let key = SigningKey::random(&mut OsRng);
        let public_pem = key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        let issued = issue(&public_pem);
        Arc::new(Self {
            key,
            chain: format!("{}\n{}", issued.certificate_pem, authority.ca_pem()),
        })
    }
}

impl DeviceSigner for P256Signer {
    fn alg(&self) -> SignerAlg {
        SignerAlg::Es256
    }

    fn cert_chain(&self) -> Result<String, CoreError> {
        Ok(self.chain.clone())
    }

    fn sign(&self, data: Vec<u8>) -> Result<Vec<u8>, CoreError> {
        let signature: Signature = self.key.sign(&data);
        Ok(signature.to_bytes().to_vec())
    }
}

pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "delta-sync-core-{name}-{}-{:x}",
            std::process::id(),
            OsRng.next_u64()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
