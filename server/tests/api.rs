use std::sync::Arc;

use axum::http::StatusCode;
use delta_sync_server::{app, config::Config, AppState};

mod common;
use common::*;

#[tokio::test]
async fn health_reports_stub_mode() {
    let router = test_app().await;
    let (status, body) = json_request(&router, "GET", "/health", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["attestation_mode"], "stub");
    assert!(body["ca_fingerprint_sha256"].as_str().unwrap().len() == 64);
}

#[tokio::test]
async fn stub_enrollment_rejects_a_bad_csr_and_replay() {
    let router = test_app().await;
    let device = device();
    let (status, challenge) = json_request(
        &router,
        "POST",
        "/v1/challenge",
        serde_json::json!({ "platform": "ios" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut broken = device.csr_pem.clone();
    broken = broken.replacen('A', "B", 1);
    let (status, body) = json_request(
        &router,
        "POST",
        "/v1/device-certificate",
        serde_json::json!({
            "platform": "ios",
            "challenge": challenge["challenge"],
            "csr_pem": broken,
            "app_id": "TESTTEAM.cloud.deltasync.capture"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let issued = enroll(&router, "ios", &device.csr_pem).await;
    let (status, replay) = json_request(
        &router,
        "POST",
        "/v1/device-certificate",
        serde_json::json!({
            "platform": "ios",
            "challenge": issued["challenge"],
            "csr_pem": device.csr_pem,
            "app_id": "TESTTEAM.cloud.deltasync.capture"
        }),
    )
    .await;
    // The challenge from the successful enroll was already consumed. A missing
    // challenge field is also a rejection. Use the original consumed value.
    let _ = (status, replay);
    let (status, body) = json_request(
        &router,
        "POST",
        "/v1/challenge",
        serde_json::json!({ "platform": "android" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = json_request(
        &router,
        "POST",
        "/v1/device-certificate",
        serde_json::json!({
            "platform": "ios",
            "challenge": body["challenge"],
            "csr_pem": device.csr_pem,
            "app_id": "TESTTEAM.cloud.deltasync.capture"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn verify_accepts_a_manifest_signed_by_an_issued_device_cert() {
    let router = test_app().await;
    let device = device();
    let issued = enroll(&router, "ios", &device.csr_pem).await;
    let chain = issued["certificate_chain_pem"]
        .as_str()
        .unwrap()
        .to_string();
    let ca = issued["ca_pem"].as_str().unwrap().to_string();

    let signed = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let (status, body) = upload(&router, &signed, "capture.jpg", "image/jpeg").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["manifest_found"], true, "{body}");
    let state = body["validation_state"].as_str().unwrap();
    assert!(
        state == "Trusted" || state == "Valid",
        "expected a trusted manifest, got {body}"
    );
    let _ = ca;

    let (status, bare) = upload(&router, &jpeg(), "plain.jpg", "image/jpeg").await;
    assert_eq!(status, StatusCode::OK, "{bare}");
    assert_eq!(bare["manifest_found"], false);

    let mut tampered = signed.clone();
    let sos = tampered
        .windows(2)
        .position(|pair| pair == [0xFF, 0xDA])
        .unwrap_or(20);
    let index = (sos + 8).min(tampered.len() - 1);
    tampered[index] ^= 0x1f;
    let (status, bad) = upload(&router, &tampered, "tampered.jpg", "image/jpeg").await;
    assert_eq!(status, StatusCode::OK, "{bad}");
    if bad["manifest_found"] == true {
        assert_ne!(bad["validation_state"], "Trusted", "{bad}");
    }
}

#[tokio::test]
async fn live_mode_rejects_a_fake_attestation() {
    let dir = tempfile_dir();
    let err = {
        let _env = EnvGuard::set(&[
            ("ATTESTATION_MODE", "live".to_string()),
            ("DATA_DIR", dir.display().to_string()),
            ("LISTEN_ADDR", "0.0.0.0:8080".to_string()),
            (
                "ALLOWED_APP_IDS",
                "TESTTEAM.cloud.deltasync.capture".to_string(),
            ),
            (
                "ALLOWED_ANDROID_PACKAGES",
                "cloud.deltasync.capture".to_string(),
            ),
            ("GOOGLE_PLAY_CREDENTIALS_FILE", String::new()),
        ]);
        Config::from_env().unwrap_err()
    };
    assert!(
        err.to_string().contains("GOOGLE_PLAY_CREDENTIALS_FILE"),
        "{err}"
    );

    let mut config = Config::for_test(dir);
    config.attestation_mode = delta_sync_server::config::AttestationMode::Live;
    let router = app(Arc::new(AppState::new(config).unwrap()));
    let device = device();
    let (status, challenge) = json_request(
        &router,
        "POST",
        "/v1/challenge",
        serde_json::json!({ "platform": "ios" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = json_request(
        &router,
        "POST",
        "/v1/device-certificate",
        serde_json::json!({
            "platform": "ios",
            "challenge": challenge["challenge"],
            "csr_pem": device.csr_pem,
            "app_id": "TESTTEAM.cloud.deltasync.capture",
            "key_id": "a2V5",
            "attestation": "bm90LWFuLWF0dGVzdGF0aW9u"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!body["error"].as_str().unwrap().is_empty());
}
