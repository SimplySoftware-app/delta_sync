use crate::edit::Edit;
use crate::error::CoreError;
use crate::keys;
use crate::sign::{sign_capture, sign_edit, verify_bytes};
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
