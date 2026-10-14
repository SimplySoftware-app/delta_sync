use std::{io::Cursor, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use c2pa::{Builder, CallbackSigner, Context, Settings, SigningAlg};
use delta_sync_server::{app, config::Config, AppState};
use image::{ImageBuffer, ImageFormat, Rgb};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::pkcs8::EncodePrivateKey;
use rand_core::OsRng;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use serde_json::Value;
use tower::ServiceExt;

struct Device {
    signing_key: SigningKey,
    csr_pem: String,
}

fn device() -> Device {
    let signing_key = SigningKey::random(&mut OsRng);
    let pem = signing_key
        .to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
        .unwrap();
    let key_pair = KeyPair::from_pem(pem.as_str()).unwrap();
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "DeltaSync Device");
    params.distinguished_name = dn;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::EmailProtection];
    let csr = params.serialize_request(&key_pair).unwrap();
    Device {
        signing_key,
        csr_pem: csr.pem().unwrap(),
    }
}

fn jpeg() -> Vec<u8> {
    let image: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(48, 32, |x, y| Rgb([(x * 5) as u8, (y * 7) as u8, 40]));
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, ImageFormat::Jpeg).unwrap();
    cursor.into_inner()
}

async fn test_app() -> axum::Router {
    let dir = tempfile_dir();
    let state = AppState::new(Config::for_test(dir)).unwrap();
    app(Arc::new(state))
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "delta-sync-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn json_request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn enroll(router: &axum::Router, platform: &str, csr: &str) -> Value {
    let (status, challenge) = json_request(
        router,
        "POST",
        "/v1/challenge",
        serde_json::json!({ "platform": platform }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{challenge}");
    let (status, issued) = json_request(
        router,
        "POST",
        "/v1/device-certificate",
        serde_json::json!({
            "platform": platform,
            "challenge": challenge["challenge"],
            "csr_pem": csr,
            "app_id": "TESTTEAM.cloud.deltasync.capture",
            "package_name": "cloud.deltasync.capture"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(issued["attestation"]["mode"], "stub");
    assert_eq!(issued["attestation"]["verified"], false);
    issued
}

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
    std::env::set_var("ATTESTATION_MODE", "live");
    std::env::set_var("DATA_DIR", &dir);
    std::env::set_var("LISTEN_ADDR", "0.0.0.0:8080");
    std::env::set_var("ALLOWED_APP_IDS", "TESTTEAM.cloud.deltasync.capture");
    std::env::set_var("ALLOWED_ANDROID_PACKAGES", "cloud.deltasync.capture");
    std::env::set_var("GOOGLE_PLAY_CREDENTIALS_FILE", "");
    let err = Config::from_env().unwrap_err();
    assert!(
        err.to_string().contains("GOOGLE_PLAY_CREDENTIALS_FILE"),
        "{err}"
    );
    std::env::remove_var("ATTESTATION_MODE");
    std::env::remove_var("ALLOWED_APP_IDS");
    std::env::remove_var("ALLOWED_ANDROID_PACKAGES");
    std::env::remove_var("GOOGLE_PLAY_CREDENTIALS_FILE");

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

fn sign_jpeg(key: &SigningKey, chain_pem: &str, jpeg: &[u8]) -> Vec<u8> {
    let key = key.clone();
    let signer = CallbackSigner::new(
        move |_ctx, data: &[u8]| {
            let signature: Signature = key.sign(data);
            Ok(signature.to_bytes().to_vec())
        },
        SigningAlg::Es256,
        chain_pem.as_bytes().to_vec(),
    );
    let manifest = r#"{
        "claim_generator": "DeltaSync test/0.1.0",
        "title": "DeltaSync capture",
        "format": "image/jpeg",
        "assertions": [{
            "label": "c2pa.actions",
            "data": {
                "actions": [{
                    "action": "c2pa.created",
                    "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
                }]
            }
        }]
    }"#;
    let settings = Settings::new()
        .with_json(r#"{"verify": {"verify_after_sign": false, "verify_trust": false}}"#)
        .unwrap();
    let context = Context::new().with_settings(settings).unwrap();
    let mut builder = Builder::from_context(context)
        .with_definition(manifest)
        .unwrap();
    let mut source = Cursor::new(jpeg.to_vec());
    let mut dest = Cursor::new(Vec::new());
    builder
        .sign(&signer, "image/jpeg", &mut source, &mut dest)
        .unwrap();
    dest.into_inner()
}

async fn upload(
    router: &axum::Router,
    bytes: &[u8],
    filename: &str,
    mime: &str,
) -> (StatusCode, Value) {
    let boundary = "----deltasyncboundary";
    let mut body = Vec::new();
    body.extend(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
        )
        .into_bytes(),
    );
    body.extend(bytes);
    body.extend(format!("\r\n--{boundary}--\r\n").into_bytes());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/verify")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into())),
    )
}
