use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use delta_sync_core::LabelVerdict;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{
    attest::{self, app_attest::APPLE_APP_ATTEST_ROOT_PEM},
    ca::verify_csr_and_point,
    challenge::{integrity_nonce, Platform},
    config::AttestationMode,
    error::ApiError,
    verify_asset::{self, sniff_format},
    AppState,
};

/// Two 32 MiB assets plus the multipart framing of `/internal/seal`.
const MAX_UPLOAD: usize = 70 * 1024 * 1024;

pub fn router(state: Arc<AppState>) -> Router {
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/v1/challenge", post(challenge))
        .route("/v1/device-certificate", post(issue_certificate))
        .route("/v1/verify", post(verify_label))
        .route("/verify", post(verify))
        .route("/internal/seal", post(crate::seal::seal));
    // The migration route for clients that still send a bare public key and a
    // stub attestation. It does not exist outside ATTESTATION_MODE=stub.
    if state.config.attestation_mode == AttestationMode::Stub {
        router = router.route("/v1/enroll", post(enroll_stub));
    }
    router
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
        .layer(RequestBodyLimitLayer::new(MAX_UPLOAD))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "delta-sync",
        "attestation_mode": state.config.attestation_mode.as_str(),
        "ca_fingerprint_sha256": state.ca.fingerprint_sha256(),
        "device_ica_fingerprint_sha256": state.ca.device_ica_fingerprint_sha256(),
        "service_ica_fingerprint_sha256": state.ca.service_ica_fingerprint_sha256(),
        "seal_enabled": state.config.seal_token.is_some(),
        "tsa_enabled": state.config.tsa_url.is_some(),
    }))
}

#[derive(Deserialize)]
struct ChallengeRequest {
    platform: String,
}

async fn challenge(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChallengeRequest>,
) -> Result<Json<Value>, ApiError> {
    let platform = Platform::parse(&body.platform)
        .ok_or_else(|| ApiError::bad_request("platform must be ios or android"))?;
    let value = state.challenges.issue(platform);
    Ok(Json(json!({
        "challenge": value,
        "platform": platform.as_str(),
        "expires_in_seconds": 300,
        "binding": "SHA-256(challenge_utf8 || 0x00 || uncompressed_p256_point)",
    })))
}

#[derive(Deserialize)]
struct IssueRequest {
    platform: String,
    challenge: String,
    csr_pem: String,
    app_id: Option<String>,
    package_name: Option<String>,
    key_id: Option<String>,
    attestation: Option<String>,
    integrity_token: Option<String>,
}

#[derive(Serialize)]
struct AttestationOutcome {
    mode: &'static str,
    verified: bool,
    detail: String,
}

async fn issue_certificate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IssueRequest>,
) -> Result<Json<Value>, ApiError> {
    let platform = Platform::parse(&body.platform)
        .ok_or_else(|| ApiError::bad_request("platform must be ios or android"))?;
    state
        .challenges
        .consume(&body.challenge, platform)
        .map_err(ApiError::bad_request)?;

    let point = verify_csr_and_point(&body.csr_pem).map_err(ApiError::bad_request)?;
    let app_identity = match platform {
        Platform::Ios => body
            .app_id
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ApiError::bad_request("ios enrollment requires app_id"))?,
        Platform::Android => body
            .package_name
            .clone()
            .or_else(|| body.app_id.clone())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ApiError::bad_request("android enrollment requires package_name"))?,
    };

    let outcome = match state.config.attestation_mode {
        AttestationMode::Stub => AttestationOutcome {
            mode: "stub",
            verified: false,
            detail: "ATTESTATION_MODE=stub. App Attest and Play Integrity were not checked.".into(),
        },
        AttestationMode::Live => match platform {
            Platform::Ios => verify_ios(&state, &body, &point, &app_identity).await?,
            Platform::Android => verify_android(&state, &body, &point, &app_identity).await?,
        },
    };

    let issued = state
        .ca
        .issue(
            &body.csr_pem,
            platform,
            &app_identity,
            state.config.device_cert_ttl,
        )
        .map_err(ApiError::internal)?;

    Ok(Json(json!({
        "certificate_pem": issued.certificate_pem,
        "certificate_chain_pem": issued.certificate_chain_pem,
        "ca_pem": state.ca.root_pem(),
        "device_ica_pem": state.ca.device_ica_pem(),
        "expires_at": issued.expires_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_else(|_| "unknown".into()),
        "serial_hex": issued.serial_hex,
        "attestation": outcome,
    })))
}

async fn verify_ios(
    state: &AppState,
    body: &IssueRequest,
    point: &[u8],
    app_id: &str,
) -> Result<AttestationOutcome, ApiError> {
    if !state.config.allowed_app_ids.iter().any(|id| id == app_id) {
        return Err(ApiError::bad_request("app_id is not in ALLOWED_APP_IDS"));
    }
    let attestation = body
        .attestation
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::bad_request("live mode requires the App Attest attestation"))?;
    let key_id = body
        .key_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::bad_request("live mode requires the App Attest key_id"))?;
    let result = attest::app_attest::verify(
        attestation,
        &body.challenge,
        point,
        app_id,
        key_id,
        APPLE_APP_ATTEST_ROOT_PEM,
    )
    .map_err(ApiError::bad_request)?;
    Ok(AttestationOutcome {
        mode: "live",
        verified: true,
        detail: format!(
            "App Attest chain verified ({}); receipt was not sent to Apple",
            result.environment
        ),
    })
}

async fn verify_android(
    state: &AppState,
    body: &IssueRequest,
    point: &[u8],
    package_name: &str,
) -> Result<AttestationOutcome, ApiError> {
    if !state
        .config
        .allowed_android_packages
        .iter()
        .any(|id| id == package_name)
    {
        return Err(ApiError::bad_request(
            "package_name is not in ALLOWED_ANDROID_PACKAGES",
        ));
    }
    let token = body
        .integrity_token
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::bad_request("live mode requires a Play Integrity token"))?;
    let Some(creds) = &state.config.google_credentials_file else {
        return Err(ApiError::NotConfigured(
            "GOOGLE_PLAY_CREDENTIALS_FILE is not set".into(),
        ));
    };
    let expected = integrity_nonce(&body.challenge, point);
    let result = attest::play_integrity::verify(&state.http, creds, package_name, token, &expected)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(AttestationOutcome {
        mode: "live",
        verified: true,
        detail: format!(
            "Play Integrity {} {:?}",
            result.app_recognition, result.device_recognition
        ),
    })
}

async fn verify(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut bytes = None;
    let mut filename = String::from("upload.bin");
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError::bad_request(err.to_string()))?
    {
        if field.name() == Some("file") {
            if let Some(name) = field.file_name() {
                filename = name.to_string();
            }
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|err| ApiError::bad_request(err.to_string()))?,
            );
        }
    }
    let bytes = bytes.ok_or_else(|| ApiError::bad_request("multipart field 'file' is required"))?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("uploaded file is empty"));
    }
    let format = sniff_format(&bytes, &filename).ok_or_else(|| {
        ApiError::bad_request("unsupported file type. Send a JPEG, PNG, WebP, or MP4.")
    })?;
    let result = verify_asset::verify_bytes(&bytes, format, state.ca.root_pem())
        .map_err(ApiError::bad_request)?;
    Ok(Json(json!({
        "manifest_found": result.manifest_found,
        "format": result.format,
        "validation_state": result.validation_state,
        "active_label": result.active_label,
        "manifest": result.manifest,
        "validation_results": result.validation_results,
        "detail": result.detail,
        "trusted_ca_fingerprint_sha256": state.ca.fingerprint_sha256(),
    })))
}

/// `POST /v1/verify`: the raw asset as the body, `x-mime-type` and optionally
/// `x-self-declared-ai: true`. Answers with the HM / AI / Unknown verdict. An
/// unreadable asset is a verdict (Unknown), not an error.
async fn verify_label(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<LabelVerdict>, ApiError> {
    if body.is_empty() {
        return Err(ApiError::bad_request("the request body is empty"));
    }
    let mime = headers
        .get("x-mime-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .or_else(|| sniff_format(&body, "").map(str::to_string))
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let declared_ai = headers
        .get("x-self-declared-ai")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "true" || v == "1");
    let verdict = tokio::task::spawn_blocking(move || {
        delta_sync_core::verify_bytes(&body, &mime, state.ca.root_pem(), declared_ai)
    })
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(verdict))
}

#[derive(Deserialize)]
struct EnrollRequest {
    platform: String,
    app_id: String,
    public_key_pem: String,
    attestation: String,
}

/// Deprecated. `ATTESTATION_MODE=stub` only. Issues a device certificate for a
/// bare public key and a non-empty attestation blob that nothing checks. Use
/// `/v1/challenge` and `/v1/device-certificate`.
async fn enroll_stub(
    State(state): State<Arc<AppState>>,
    Json(body): Json<EnrollRequest>,
) -> Result<Json<Value>, ApiError> {
    if Platform::parse(&body.platform).is_none() {
        return Err(ApiError::bad_request("platform must be ios or android"));
    }
    if body.attestation.trim().is_empty() {
        return Err(ApiError::Forbidden("attestation is required".into()));
    }
    if body.app_id.trim().is_empty() || body.public_key_pem.trim().is_empty() {
        return Err(ApiError::bad_request(
            "app_id and public_key_pem are required",
        ));
    }
    let issued = state
        .ca
        .issue_for_public_key(
            &body.public_key_pem,
            &body.platform,
            body.app_id.trim(),
            state.config.device_cert_ttl,
        )
        .map_err(ApiError::bad_request)?;
    Ok(Json(json!({
        "certificate_pem": issued.certificate_pem,
        "certificate_chain_pem": issued.certificate_chain_pem,
        "ca_pem": state.ca.root_pem(),
        "device_ica_pem": state.ca.device_ica_pem(),
        "expires_at_unix": issued.expires_at.unix_timestamp(),
        "deprecated": "use /v1/challenge and /v1/device-certificate",
    })))
}
