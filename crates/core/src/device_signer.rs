//! Platform-held signing keys.
//!
//! The private key never reaches Rust. Rust builds the C2PA claim and hands
//! the bytes to be signed to a [`DeviceSigner`] implemented in Swift (Secure
//! Enclave), Kotlin (Android Keystore, StrongBox with TEE fallback) or, in
//! tests, Rust.

#[cfg(feature = "c2pa")]
use std::sync::Arc;

use crate::error::CoreError;

/// Signature algorithm of a [`DeviceSigner`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SignerAlg {
    /// ECDSA over P-256 with SHA-256. Hardware keys use this one.
    Es256,
    /// Ed25519. Only for software keys (tests, the legacy dev CA).
    Ed25519,
}

/// A signing key that lives outside Rust.
///
/// Implementations must be thread safe: the signer may be called from any
/// thread that runs a signing API.
#[cfg_attr(feature = "ffi", uniffi::export(with_foreign))]
pub trait DeviceSigner: Send + Sync {
    /// Algorithm of the key. Constant for the lifetime of the signer.
    fn alg(&self) -> SignerAlg;

    /// PEM certificate chain: the device certificate first, then the
    /// intermediates up to (not necessarily including) the root.
    fn cert_chain(&self) -> Result<String, CoreError>;

    /// Sign `data`. For [`SignerAlg::Es256`] return the raw 64-byte `r || s`
    /// (IEEE P1363) signature that COSE expects, not an ASN.1 DER one. The
    /// hashing (SHA-256) is part of the signing operation.
    fn sign(&self, data: Vec<u8>) -> Result<Vec<u8>, CoreError>;
}

#[cfg(feature = "ffi")]
impl From<uniffi::UnexpectedUniFFICallbackError> for CoreError {
    fn from(err: uniffi::UnexpectedUniFFICallbackError) -> Self {
        CoreError::msg(format!("signer callback failed: {err}"))
    }
}

#[cfg(feature = "c2pa")]
pub(crate) fn c2pa_signer(
    signer: Arc<dyn DeviceSigner>,
) -> Result<c2pa::CallbackSigner, CoreError> {
    use c2pa::{CallbackSigner, SigningAlg};

    let chain = signer.cert_chain()?;
    if !chain.contains("BEGIN CERTIFICATE") {
        return Err(CoreError::msg(
            "DeviceSigner.cert_chain must return a PEM certificate chain",
        ));
    }
    let alg = match signer.alg() {
        SignerAlg::Es256 => SigningAlg::Es256,
        SignerAlg::Ed25519 => SigningAlg::Ed25519,
    };
    // Both algorithms produce a fixed 64-byte signature in COSE form.
    let callback = move |_ctx: *const (), data: &[u8]| {
        let signature = signer
            .sign(data.to_vec())
            .map_err(|e| c2pa::Error::BadParam(e.to_string()))?;
        if signature.len() != 64 {
            return Err(c2pa::Error::BadParam(format!(
                "DeviceSigner.sign returned {} bytes, expected 64: \
                 ES256 must be raw r||s, not DER",
                signature.len()
            )));
        }
        Ok(signature)
    };
    Ok(CallbackSigner::new(callback, alg, chain.into_bytes()))
}
