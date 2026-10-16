//! Helpers shared by the integration tests. Keys and certificates are generated
//! at run time; nothing here is a real credential.
#![allow(dead_code)]

use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

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

pub struct Device {
    pub signing_key: SigningKey,
    pub csr_pem: String,
}

pub fn device() -> Device {
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
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::Other(vec![
        1, 3, 6, 1, 4, 1, 62558, 2, 1,
    ])];
    let csr = params.serialize_request(&key_pair).unwrap();
    Device {
        signing_key,
        csr_pem: csr.pem().unwrap(),
    }
}

pub fn jpeg() -> Vec<u8> {
    jpeg_sized(48, 32)
}

pub fn jpeg_sized(width: u32, height: u32) -> Vec<u8> {
    let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([(x * 5) as u8, (y * 7) as u8, 40])
    });
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, ImageFormat::Jpeg).unwrap();
    cursor.into_inner()
}

pub async fn test_app() -> axum::Router {
    let (router, _) = test_app_with(|_| {});
    router
}

/// Build an app on a fresh data dir after letting the caller adjust the config.
pub fn test_app_with(adjust: impl FnOnce(&mut Config)) -> (axum::Router, Arc<AppState>) {
    let mut config = Config::for_test(tempfile_dir());
    adjust(&mut config);
    let state = Arc::new(AppState::new(config).unwrap());
    (app(state.clone()), state)
}

/// Unique per call: pid + process-wide counter, so parallel tests never share
/// a data dir (a clock-based name can collide and fail with "File exists").
pub fn tempfile_dir() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "delta-sync-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Serializes tests that mutate process environment variables and restores
/// them on drop, even if the test panics.
static ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    keys: Vec<&'static str>,
}

impl EnvGuard {
    pub fn set(vars: &[(&'static str, String)]) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for (key, value) in vars {
            std::env::set_var(key, value);
        }
        Self {
            _lock: lock,
            keys: vars.iter().map(|(key, _)| *key).collect(),
        }
    }

    /// Hold the lock with these variables removed.
    pub fn unset(keys: &[&'static str]) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for key in keys {
            std::env::remove_var(key);
        }
        Self {
            _lock: lock,
            keys: keys.to_vec(),
        }
    }

    /// Set another variable under the same lock. It is removed on drop too.
    pub fn also_set(&mut self, key: &'static str, value: &str) {
        std::env::set_var(key, value);
        if !self.keys.contains(&key) {
            self.keys.push(key);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            std::env::remove_var(key);
        }
    }
}

pub async fn json_request(
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

pub async fn enroll(router: &axum::Router, platform: &str, csr: &str) -> Value {
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

/// Enroll a fresh device and return it with the chain it signs with.
pub async fn enrolled_device(router: &axum::Router) -> (Device, String, String) {
    let device = device();
    let issued = enroll(router, "ios", &device.csr_pem).await;
    let chain = issued["certificate_chain_pem"]
        .as_str()
        .unwrap()
        .to_string();
    let root = issued["ca_pem"].as_str().unwrap().to_string();
    (device, chain, root)
}

pub fn sign_jpeg(key: &SigningKey, chain_pem: &str, jpeg: &[u8]) -> Vec<u8> {
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

/// Multipart body with `(name, content type, bytes)` parts.
pub fn multipart(boundary: &str, parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, mime, bytes) in parts {
        let mut head = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"");
        if let Some(mime) = mime {
            head.push_str(&format!("; filename=\"asset\"\r\nContent-Type: {mime}"));
        }
        head.push_str("\r\n\r\n");
        body.extend(head.into_bytes());
        body.extend(*bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{boundary}--\r\n").into_bytes());
    body
}

pub async fn raw_request(
    router: &axum::Router,
    uri: &str,
    headers: &[(&str, &str)],
    body: Vec<u8>,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder().method("POST").uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

pub async fn upload(
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
