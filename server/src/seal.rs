//! Internal media-processor API: `POST /internal/seal`.
//!
//! The media processor sends the device-signed original, the bytes it rendered
//! from it, and the edits it applied. The service answers with the rendered bytes
//! carrying a new manifest, signed with the Service ICA's certificate. That
//! manifest holds the edit actions, `c2pa.transcoded` (and `c2pa.resized`), and
//! the original as its `parentOf` ingredient.
//!
//! A seal attests to processing, never to camera capture. The service only seals
//! an origin that verifies as a trusted HM capture signed under the Device ICA.
//! Anything else is refused, so a seal can never turn an unsigned or foreign
//! file into HM. The service key is the one in `DATA_DIR` (or the configured
//! path). It is never generated per request.

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use delta_sync_core::{
    seal_derivative, verify_bytes_with, CoreError, DeviceSigner, Edit, SealRequest, SignerAlg,
    Timestamping, VerifyOptions,
};
use p256::{
    ecdsa::{signature::Signer, Signature, SigningKey},
    pkcs8::DecodePrivateKey,
};
use serde_json::json;
use subtle::ConstantTimeEq;

use crate::{ca::CertificateAuthority, AppState};

pub const MAX_ASSET_BYTES: usize = 32 * 1024 * 1024;
const MAX_EDITS_BYTES: usize = 8 * 1024;
const MAX_EDITS: usize = 8;
const PERMIT_WAIT: Duration = Duration::from_secs(30);
/// Claim generator written into derivative manifests.
pub use delta_sync_core::SEAL_GENERATOR;

const ALLOWED_MIMES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/webp",
    "video/mp4",
    "video/quicktime",
];

pub struct SealInput<'a> {
    pub original: &'a [u8],
    pub original_mime: &'a str,
    pub rendered: &'a [u8],
    pub rendered_mime: &'a str,
    pub declared_ai: bool,
    pub edits: &'a [Edit],
    /// The rendered asset has fewer pixels than the original.
    pub resized: bool,
}

/// True when `bytes` verifies as an HM capture under our root **and** the only
/// signer in its chain is the Device ICA. A seal signed by the Service ICA (a
/// derivative of a capture) does not pass, and neither does anything signed
/// under another authority.
pub fn is_trusted_device_capture(options: &VerifyOptions, bytes: &[u8], mime: &str) -> bool {
    let verdict = verify_bytes_with(bytes, mime, options, false);
    verdict.label == "HM"
        && verdict.trusted
        && !verdict.signers.is_empty()
        && verdict.signers.iter().all(|role| role == "device")
}

/// The service certificate as a [`DeviceSigner`]: the key is the one loaded
/// from `DATA_DIR`, never one made for the request.
struct ServiceSigner {
    key: SigningKey,
    chain: String,
}

impl ServiceSigner {
    fn new(ca: &CertificateAuthority) -> anyhow::Result<Self> {
        Ok(Self {
            key: SigningKey::from_pkcs8_pem(ca.service_key_pem())
                .map_err(|err| anyhow::anyhow!("service key: {err}"))?,
            chain: ca.service_chain_pem().to_string(),
        })
    }
}

impl DeviceSigner for ServiceSigner {
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

/// Sign `rendered` as a derivative of `original` with the service certificate.
/// The manifest, the actions and the video Merkle hashing come from the core.
pub fn sign_derivative(
    ca: &CertificateAuthority,
    options: &VerifyOptions,
    tsa_url: Option<&str>,
    input: &SealInput<'_>,
) -> anyhow::Result<Vec<u8>> {
    let signer = Arc::new(ServiceSigner::new(ca)?);
    let timestamping = match tsa_url {
        Some(url) => Timestamping::Url(url.to_string()),
        None => Timestamping::None,
    };
    let request = SealRequest {
        original: input.original,
        original_mime: input.original_mime,
        rendered: input.rendered,
        rendered_mime: input.rendered_mime,
        declared_ai: input.declared_ai,
        edits: input.edits,
        resized: input.resized,
    };
    seal_derivative(&request, signer, &timestamping, options)
        .map_err(|err| anyhow::anyhow!("sign: {err}"))
}

#[derive(Debug)]
pub enum SealError {
    NotConfigured,
    Unauthorized,
    Busy,
    BadRequest(&'static str),
    TooLarge,
    UnsupportedMedia,
    /// The origin is not a trusted HM capture under the Device ICA.
    OriginRefused,
    /// Signing failed for a reason in the inputs.
    Unprocessable(String),
    /// The time-stamp authority did not answer.
    TimestampUnavailable(String),
    Internal,
}

impl IntoResponse for SealError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "sealing is not configured".to_string(),
            ),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
            Self::Busy => (StatusCode::SERVICE_UNAVAILABLE, "seal queue is full".into()),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, (*message).to_string()),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "asset too large".into()),
            Self::UnsupportedMedia => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported media type".into(),
            ),
            Self::OriginRefused => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "origin is not a trusted capture under the Device ICA".into(),
            ),
            Self::Unprocessable(message) => (StatusCode::UNPROCESSABLE_ENTITY, message.clone()),
            Self::TimestampUnavailable(message) => (StatusCode::BAD_GATEWAY, message.clone()),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_string(),
            ),
        };
        (status, axum::Json(json!({ "error": message }))).into_response()
    }
}

pub async fn seal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut parts: Multipart,
) -> Result<Response, SealError> {
    let expected = state
        .config
        .seal_token
        .as_deref()
        .ok_or(SealError::NotConfigured)?;
    let supplied = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("");
    if !bool::from(supplied.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(SealError::Unauthorized);
    }
    let declared_ai = headers
        .get("x-self-declared-ai")
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| h == "true" || h == "1");

    let _permit = tokio::time::timeout(PERMIT_WAIT, state.seal_permits.acquire())
        .await
        .map_err(|_| SealError::Busy)?
        .map_err(|_| SealError::Internal)?;

    let mut original = None;
    let mut rendered = None;
    let mut edits: Vec<Edit> = vec![];
    let mut resized = false;
    while let Some(part) = parts
        .next_field()
        .await
        .map_err(|_| SealError::BadRequest("malformed multipart body"))?
    {
        let name = part.name().unwrap_or("").to_owned();
        match name.as_str() {
            "edits" => {
                let bytes = part
                    .bytes()
                    .await
                    .map_err(|_| SealError::BadRequest("unreadable edits field"))?;
                if bytes.len() > MAX_EDITS_BYTES {
                    return Err(SealError::TooLarge);
                }
                edits = serde_json::from_slice(&bytes)
                    .map_err(|_| SealError::BadRequest("edits must be a JSON array of edits"))?;
                if edits.len() > MAX_EDITS {
                    return Err(SealError::BadRequest("too many edits"));
                }
            }
            "resized" => {
                let bytes = part
                    .bytes()
                    .await
                    .map_err(|_| SealError::BadRequest("unreadable resized field"))?;
                resized = matches!(bytes.as_ref(), b"true" | b"1");
            }
            "original" | "rendered" => {
                let mime = part.content_type().unwrap_or("").to_owned();
                if !ALLOWED_MIMES.contains(&mime.as_str()) {
                    return Err(SealError::UnsupportedMedia);
                }
                let data = part
                    .bytes()
                    .await
                    .map_err(|_| SealError::BadRequest("unreadable asset"))?;
                if data.is_empty() || data.len() > MAX_ASSET_BYTES {
                    return Err(SealError::TooLarge);
                }
                let slot = if name == "original" {
                    &mut original
                } else {
                    &mut rendered
                };
                if slot.is_some() {
                    return Err(SealError::BadRequest("duplicate field"));
                }
                *slot = Some((data, mime));
            }
            _ => return Err(SealError::BadRequest("unknown field")),
        }
    }
    let (original, original_mime) =
        original.ok_or(SealError::BadRequest("original is required"))?;
    let (rendered, rendered_mime) =
        rendered.ok_or(SealError::BadRequest("rendered is required"))?;

    let tsa_url = state.config.tsa_url.clone();
    let content_type = rendered_mime.clone();
    let worker = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let options = worker.verify_options();
        if !is_trusted_device_capture(&options, &original, &original_mime) {
            return Err(SealError::OriginRefused);
        }
        let input = SealInput {
            original: &original,
            original_mime: &original_mime,
            rendered: &rendered,
            rendered_mime: &rendered_mime,
            declared_ai,
            edits: &edits,
            resized,
        };
        match sign_derivative(&worker.ca, &options, tsa_url.as_deref(), &input) {
            Ok(sealed) => Ok(sealed),
            Err(err) if tsa_url.is_some() => {
                // c2pa words a TSA outage as a generic HTTP error. Telling it
                // apart from a bad input is exact: if signing without the
                // time-stamp works, the authority was the problem.
                if sign_derivative(&worker.ca, &options, None, &input).is_ok() {
                    tracing::error!(error = %err, "time-stamp authority request failed");
                    Err(SealError::TimestampUnavailable(
                        "time-stamp authority unavailable".into(),
                    ))
                } else {
                    tracing::warn!(error = %err, "seal refused");
                    Err(SealError::Unprocessable(
                        "could not seal the rendered asset".into(),
                    ))
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "seal refused");
                Err(SealError::Unprocessable(
                    "could not seal the rendered asset".into(),
                ))
            }
        }
    })
    .await
    .map_err(|_| SealError::Internal)??;

    Ok((
        [
            ("content-type", content_type),
            ("cache-control", "no-store".into()),
        ],
        result,
    )
        .into_response())
}
