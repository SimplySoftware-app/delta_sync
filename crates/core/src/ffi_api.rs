use crate::edit::Edit;
use crate::error::CoreError;
use crate::keys;
use std::path::Path;
use std::sync::Arc;

use crate::device_signer::DeviceSigner;
use crate::sign::{
    sign_capture, sign_capture_file as sign_file, sign_capture_file_with, sign_edit, verify_bytes,
    verify_file, verify_file_with, TimestampProvider, Timestamping, VerifyOptions,
};
use crate::VERSION;

#[uniffi::export]
pub fn core_version() -> String {
    VERSION.to_string()
}

#[uniffi::export]
pub fn generate_device_key_json() -> Result<String, CoreError> {
    let key = keys::generate_device_key()?;
    serde_json::to_string(&serde_json::json!({
        "private_pem": key.private_pem,
        "public_pem": key.public_pem,
    }))
    .map_err(|e| CoreError::msg(e.to_string()))
}

#[uniffi::export]
pub fn sign_capture_bytes(
    asset: Vec<u8>,
    mime: String,
    private_pem: String,
    cert_pem: String,
    ca_pem: String,
) -> Result<Vec<u8>, CoreError> {
    sign_capture(&asset, &mime, &private_pem, &cert_pem, &ca_pem)
}

#[uniffi::export]
pub fn sign_edit_bytes(
    asset: Vec<u8>,
    mime: String,
    private_pem: String,
    cert_pem: String,
    ca_pem: String,
    edit_json: String,
) -> Result<Vec<u8>, CoreError> {
    let edit: Edit = serde_json::from_str(&edit_json).map_err(|e| CoreError::msg(e.to_string()))?;
    sign_edit(&asset, &mime, &private_pem, &cert_pem, &ca_pem, &edit)
}

#[uniffi::export]
pub fn verify_asset_json(
    asset: Vec<u8>,
    mime: String,
    ca_pem: String,
    self_declared_ai: bool,
) -> String {
    let verdict = verify_bytes(&asset, &mime, &ca_pem, self_declared_ai);
    serde_json::to_string(&verdict).unwrap_or_else(|_| "{\"label\":\"Unknown\"}".into())
}

/// Sign a JPEG, MOV or MP4 capture on disk with a platform-held key.
/// Streams from `in_path` to `out_path`; the file is never held in memory.
#[uniffi::export]
pub fn sign_capture_file(
    in_path: String,
    out_path: String,
    signer: Arc<dyn DeviceSigner>,
) -> Result<(), CoreError> {
    sign_file(Path::new(&in_path), Path::new(&out_path), signer)
}

/// Verify a JPEG, MOV or MP4 on disk without loading it into memory.
#[uniffi::export]
pub fn verify_file_json(path: String, ca_pem: String, self_declared_ai: bool) -> String {
    let verdict = verify_file(Path::new(&path), None, &ca_pem, self_declared_ai);
    serde_json::to_string(&verdict).unwrap_or_else(|_| "{\"label\":\"Unknown\"}".into())
}

/// [`sign_capture_file`] with an RFC 3161 time stamp on the signature, so the
/// capture still verifies after the short-lived device certificate expires.
/// The platform makes the request to the time-stamp authority in `provider`.
#[uniffi::export]
pub fn sign_capture_file_with_timestamp(
    in_path: String,
    out_path: String,
    signer: Arc<dyn DeviceSigner>,
    provider: Arc<dyn TimestampProvider>,
) -> Result<(), CoreError> {
    sign_capture_file_with(
        Path::new(&in_path),
        Path::new(&out_path),
        signer,
        &Timestamping::Provider(provider),
    )
}

/// [`verify_file_json`] that also trusts the time-stamp authorities whose root
/// certificates are in `tsa_anchors_pem`, so a capture made with an expired
/// device certificate still verifies when a trusted authority dated it.
#[uniffi::export]
pub fn verify_file_json_with_tsa(
    path: String,
    ca_pem: String,
    tsa_anchors_pem: String,
    self_declared_ai: bool,
) -> String {
    let options = VerifyOptions::new(ca_pem).with_tsa_anchors(tsa_anchors_pem);
    let verdict = verify_file_with(Path::new(&path), None, &options, self_declared_ai);
    serde_json::to_string(&verdict).unwrap_or_else(|_| "{\"label\":\"Unknown\"}".into())
}
