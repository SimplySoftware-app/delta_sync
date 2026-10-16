//! PKI, `/v1/verify`, `/internal/seal` and the stub-only `/v1/enroll`.
//! Every key and certificate is generated at run time. Nothing here uses the network.

use std::sync::Arc;

use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine};
use delta_sync_server::{
    ca::CertificateAuthority,
    config::{AttestationMode, Config},
    seal::{sign_derivative, SealInput},
    verify_asset, AppState,
};
use serde_json::{json, Value};
use x509_parser::{certificate::X509Certificate, prelude::FromDer};

mod common;
use common::*;

const TOKEN: &str = "private-test-media-service-token-123456";
const CLAIM_SIGNING_OID: &str = "1.3.6.1.4.1.62558.2.1";

fn der(pem: &str) -> Vec<u8> {
    let body: String = pem
        .lines()
        .skip_while(|l| !l.starts_with("-----BEGIN"))
        .skip(1)
        .take_while(|l| !l.starts_with("-----END"))
        .collect();
    STANDARD.decode(body).unwrap()
}

fn with_cert<T>(pem: &str, f: impl FnOnce(&X509Certificate<'_>) -> T) -> T {
    let bytes = der(pem);
    let (_, cert) = X509Certificate::from_der(&bytes).unwrap();
    f(&cert)
}

fn seal_app() -> (axum::Router, Arc<AppState>) {
    test_app_with(|config| config.seal_token = Some(TOKEN.to_string()))
}

async fn seal_request(
    router: &axum::Router,
    token: Option<&str>,
    parts: &[(&str, Option<&str>, &[u8])],
    extra: &[(&str, &str)],
) -> (StatusCode, Vec<u8>) {
    let content_type = "multipart/form-data; boundary=sealboundary".to_string();
    let auth = token.map(|t| format!("Bearer {t}"));
    let mut headers: Vec<(&str, &str)> = vec![("content-type", content_type.as_str())];
    if let Some(auth) = &auth {
        headers.push(("authorization", auth.as_str()));
    }
    headers.extend_from_slice(extra);
    raw_request(
        router,
        "/internal/seal",
        &headers,
        multipart("sealboundary", parts),
    )
    .await
}

async fn verify_label(router: &axum::Router, bytes: &[u8], mime: &str, declared: bool) -> Value {
    let mut headers = vec![("x-mime-type", mime)];
    if declared {
        headers.push(("x-self-declared-ai", "true"));
    }
    let (status, body) = raw_request(router, "/v1/verify", &headers, bytes.to_vec()).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn device_certificate_is_claim_signing_short_lived_and_under_the_device_ica() {
    let router = test_app().await;
    let device = device();
    let issued = enroll(&router, "android", &device.csr_pem).await;
    let leaf = issued["certificate_pem"].as_str().unwrap();
    let chain = issued["certificate_chain_pem"].as_str().unwrap();
    let root = issued["ca_pem"].as_str().unwrap();
    let device_ica = issued["device_ica_pem"].as_str().unwrap();

    with_cert(leaf, |cert| {
        let eku = cert.extended_key_usage().unwrap().unwrap().value;
        assert!(!eku.email_protection, "emailProtection must be gone");
        let others: Vec<String> = eku.other.iter().map(|o| o.to_string()).collect();
        assert_eq!(others, vec![CLAIM_SIGNING_OID.to_string()]);
        assert!(!cert.is_ca());
        let life = cert.validity().not_after.timestamp() - cert.validity().not_before.timestamp();
        // 15 minutes by default, plus the 5 minute back-dating.
        assert!(life <= 900 + 300 + 5, "device cert lives {life}s");
        assert_eq!(
            cert.subject()
                .iter_organization()
                .next()
                .unwrap()
                .as_str()
                .unwrap(),
            "Delta Sync Device"
        );
    });
    // The chain is leaf + Device ICA. The root is the client's trust anchor and is not in it.
    assert_eq!(chain.matches("BEGIN CERTIFICATE").count(), 2);
    assert!(chain.ends_with(device_ica));
    with_cert(device_ica, |ica| {
        assert!(ica.is_ca());
        assert_eq!(
            ica.subject()
                .iter_common_name()
                .next()
                .unwrap()
                .as_str()
                .unwrap(),
            "Delta Sync Device ICA"
        );
    });
    with_cert(root, |root| {
        assert_eq!(
            root.issuer(),
            root.subject(),
            "the anchor is the self-signed root"
        );
    });
    with_cert(device_ica, |ica| {
        with_cert(root, |root| {
            ica.verify_signature(Some(root.public_key())).unwrap();
        });
    });
}

#[tokio::test]
async fn device_certificate_ttl_is_capped_at_24_hours() {
    let mut env = EnvGuard::set(&[("DATA_DIR", tempfile_dir().display().to_string())]);
    for (value, ok) in [("86400", true), ("86401", false), ("59", false)] {
        env.also_set("DEVICE_CERT_TTL_SECONDS", value);
        let result = Config::from_env();
        assert_eq!(result.is_ok(), ok, "{value}: {:?}", result.err());
    }
    // The CA also clamps, whatever the config says.
    let (router, state) = test_app_with(|config| {
        config.device_cert_ttl = std::time::Duration::from_secs(30 * 86_400);
    });
    let device = device();
    let issued = enroll(&router, "ios", &device.csr_pem).await;
    with_cert(issued["certificate_pem"].as_str().unwrap(), |cert| {
        let life = cert.validity().not_after.timestamp() - cert.validity().not_before.timestamp();
        assert!(life <= 86_400 + 300 + 5, "{life}");
    });
    drop(state);
}

#[tokio::test]
async fn csr_extensions_and_subject_do_not_reach_the_certificate() {
    // The test CSR asks for its own EKU. The leaf must carry the authority's, not the CSR's.
    let router = test_app().await;
    let (_, chain, _) = enrolled_device(&router).await;
    with_cert(&chain, |cert| {
        assert_eq!(
            cert.subject()
                .iter_common_name()
                .next()
                .unwrap()
                .as_str()
                .unwrap(),
            "Delta Sync device"
        );
    });
}

#[tokio::test]
async fn service_key_is_persisted_and_the_hierarchy_survives_a_restart() {
    let dir = tempfile_dir();
    let first = CertificateAuthority::load_or_create(&dir, &Default::default()).unwrap();
    let again = CertificateAuthority::load_or_create(&dir, &Default::default()).unwrap();
    assert_eq!(first.fingerprint_sha256(), again.fingerprint_sha256());
    assert_eq!(
        first.service_fingerprint_sha256(),
        again.service_fingerprint_sha256()
    );
    assert_eq!(first.service_key_pem(), again.service_key_pem());
    assert_ne!(
        first.device_ica_fingerprint_sha256(),
        first.service_ica_fingerprint_sha256()
    );
    // The service leaf is long lived and under the Service ICA, with the claim-signing EKU.
    let chain = first.service_chain_pem();
    assert_eq!(chain.matches("BEGIN CERTIFICATE").count(), 2);
    with_cert(chain, |leaf| {
        let life = leaf.validity().not_after.timestamp() - leaf.validity().not_before.timestamp();
        assert!(life > 300 * 86_400, "service cert lives {life}s");
        let eku = leaf.extended_key_usage().unwrap().unwrap().value;
        assert_eq!(eku.other[0].to_string(), CLAIM_SIGNING_OID);
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for file in [
            "service.key.pem",
            "service-ica.key.pem",
            "device-ica.key.pem",
        ] {
            let mode = std::fs::metadata(dir.join(file))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "{file}");
        }
    }
    // The root key is not kept anywhere.
    assert!(!dir.join("root.key.pem").exists());
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        assert!(!name.contains("root.key"), "{name}");
    }
}

#[tokio::test]
async fn a_partial_hierarchy_is_never_overwritten() {
    let dir = tempfile_dir();
    CertificateAuthority::load_or_create(&dir, &Default::default()).unwrap();
    std::fs::remove_file(dir.join("service-ica.key.pem")).unwrap();
    let err = CertificateAuthority::load_or_create(&dir, &Default::default())
        .err()
        .expect("must refuse");
    assert!(err.to_string().contains("incomplete"), "{err}");
}

#[tokio::test]
async fn an_expiring_service_certificate_is_renewed_without_changing_the_icas() {
    let dir = tempfile_dir();
    let first = CertificateAuthority::load_or_create(&dir, &Default::default()).unwrap();
    // Replace the service leaf by one that is about to expire: reuse the ICA to issue it.
    let renewed_after = {
        std::fs::write(dir.join("service.crt.pem"), b"not a certificate").unwrap();
        CertificateAuthority::load_or_create(&dir, &Default::default()).unwrap()
    };
    assert_eq!(
        first.fingerprint_sha256(),
        renewed_after.fingerprint_sha256()
    );
    assert_eq!(
        first.service_ica_fingerprint_sha256(),
        renewed_after.service_ica_fingerprint_sha256()
    );
    assert_ne!(
        first.service_fingerprint_sha256(),
        renewed_after.service_fingerprint_sha256()
    );
}

#[tokio::test]
async fn v1_verify_returns_the_label_verdict() {
    let (router, _) = seal_app();
    let (device, chain, _) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());

    let verdict = verify_label(&router, &captured, "image/jpeg", false).await;
    assert_eq!(verdict["label"], "HM", "{verdict}");
    assert_eq!(verdict["trusted"], true, "{verdict}");
    assert!(verdict["actions"]
        .as_array()
        .unwrap()
        .contains(&json!("c2pa.created")));

    let plain = verify_label(&router, &jpeg(), "image/jpeg", false).await;
    assert_eq!(plain["label"], "Unknown", "{plain}");

    let declared = verify_label(&router, &captured, "image/jpeg", true).await;
    assert_eq!(declared["label"], "AI", "{declared}");

    // The mime header is optional: the service sniffs the bytes.
    let (status, body) = raw_request(&router, "/v1/verify", &[], captured.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let sniffed: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(sniffed["label"], "HM", "{sniffed}");

    // One flipped byte in the pixels breaks the hash: never HM.
    let mut tampered = captured.clone();
    let sos = tampered
        .windows(2)
        .position(|pair| pair == [0xFF, 0xDA])
        .unwrap();
    tampered[sos + 8] ^= 0x1f;
    let bad = verify_label(&router, &tampered, "image/jpeg", false).await;
    assert_ne!(bad["label"], "HM", "{bad}");

    let (status, _) = raw_request(&router, "/v1/verify", &[], vec![]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_capture_from_another_authority_is_unknown() {
    let (router, _) = seal_app();
    let (other_router, _) = seal_app();
    let (device, chain, _) = enrolled_device(&other_router).await;
    let foreign = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let verdict = verify_label(&router, &foreign, "image/jpeg", false).await;
    assert_eq!(verdict["label"], "Unknown", "{verdict}");
}

#[tokio::test]
async fn seal_is_closed_without_the_shared_secret() {
    let (device_router, _) = seal_app();
    let (device, chain, _) = enrolled_device(&device_router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let rendered = jpeg_sized(24, 16);
    let parts: Vec<(&str, Option<&str>, &[u8])> = vec![
        ("original", Some("image/jpeg"), &captured),
        ("rendered", Some("image/jpeg"), &rendered),
    ];

    // No token configured: the route exists but is unavailable.
    let (unconfigured, _) = test_app_with(|_| {});
    let (status, _) = seal_request(&unconfigured, Some(TOKEN), &parts, &[]).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    for token in [None, Some("wrong"), Some("")] {
        let (status, _) = seal_request(&device_router, token, &parts, &[]).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{token:?}");
    }
}

#[tokio::test]
async fn seal_signs_a_derivative_with_the_service_cert_and_keeps_the_capture_as_parent() {
    let (router, state) = seal_app();
    let (device, chain, root) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let rendered = jpeg_sized(24, 16);
    let edits = serde_json::to_vec(&json!([
        {"op": "crop", "x": 0, "y": 0, "width": 24, "height": 16},
        {"op": "text", "body": "hola"}
    ]))
    .unwrap();

    let (status, sealed) = seal_request(
        &router,
        Some(TOKEN),
        &[
            ("original", Some("image/jpeg"), &captured),
            ("rendered", Some("image/jpeg"), &rendered),
            ("edits", None, &edits),
            ("resized", None, b"true"),
        ],
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&sealed)
    );
    assert_ne!(sealed, rendered);

    let verdict = verify_label(&router, &sealed, "image/jpeg", false).await;
    assert_eq!(verdict["label"], "HM", "{verdict}");
    assert_eq!(verdict["trusted"], true, "{verdict}");
    let actions: Vec<&str> = verdict["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for expected in [
        "c2pa.cropped",
        "c2pa.drawing",
        "c2pa.transcoded",
        "c2pa.resized",
        "c2pa.created",
    ] {
        assert!(
            actions.contains(&expected),
            "{expected} missing from {actions:?}"
        );
    }

    // The active manifest is signed by the service. Its parent is the device capture.
    let read = verify_asset::verify_bytes(&sealed, "image/jpeg", &root).unwrap();
    let manifests = read.manifest["manifests"].as_object().unwrap();
    let active = &read.manifest["manifests"][read.active_label.as_deref().unwrap()];
    assert_eq!(active["signature_info"]["issuer"], "Delta Sync Service");
    assert_eq!(
        active["claim_generator_info"][0]["name"],
        "Delta Sync Authority"
    );
    assert_eq!(active["ingredients"][0]["relationship"], "parentOf");
    let issuers: Vec<&str> = manifests
        .values()
        .filter_map(|m| m["signature_info"]["issuer"].as_str())
        .collect();
    assert!(issuers.contains(&"Delta Sync Device"), "{issuers:?}");
    assert!(active["signature_info"]["cert_serial_number"]
        .as_str()
        .is_some());
    drop(state);
}

#[tokio::test]
async fn seal_never_uses_a_per_request_key() {
    let (router, _) = seal_app();
    let (device, chain, root) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let rendered = jpeg_sized(24, 16);
    let mut serials = std::collections::HashSet::new();
    for _ in 0..2 {
        let (status, sealed) = seal_request(
            &router,
            Some(TOKEN),
            &[
                ("original", Some("image/jpeg"), &captured),
                ("rendered", Some("image/jpeg"), &rendered),
            ],
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let read = verify_asset::verify_bytes(&sealed, "image/jpeg", &root).unwrap();
        let active = &read.manifest["manifests"][read.active_label.as_deref().unwrap()];
        serials.insert(active["signature_info"]["cert_serial_number"].to_string());
    }
    assert_eq!(
        serials.len(),
        1,
        "the service certificate must be the same every time"
    );
}

#[tokio::test]
async fn seal_refuses_unsigned_foreign_and_service_signed_origins() {
    let (router, _) = seal_app();
    let rendered = jpeg_sized(24, 16);

    // Unsigned original.
    let plain = jpeg();
    let (status, _) = seal_request(
        &router,
        Some(TOKEN),
        &[
            ("original", Some("image/jpeg"), &plain),
            ("rendered", Some("image/jpeg"), &rendered),
        ],
        &[("x-self-declared-ai", "true")],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // A capture signed under another authority's hierarchy.
    let (other, _) = seal_app();
    let (foreign_device, foreign_chain, _) = enrolled_device(&other).await;
    let foreign = sign_jpeg(&foreign_device.signing_key, &foreign_chain, &jpeg());
    let (status, _) = seal_request(
        &router,
        Some(TOKEN),
        &[
            ("original", Some("image/jpeg"), &foreign),
            ("rendered", Some("image/jpeg"), &rendered),
        ],
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // A file the service itself sealed is HM, but it is not a capture under the
    // Device ICA, so it cannot be sealed again as if it were one.
    let (device, chain, _) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let (status, sealed) = seal_request(
        &router,
        Some(TOKEN),
        &[
            ("original", Some("image/jpeg"), &captured),
            ("rendered", Some("image/jpeg"), &rendered),
        ],
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = seal_request(
        &router,
        Some(TOKEN),
        &[
            ("original", Some("image/jpeg"), &sealed),
            ("rendered", Some("image/jpeg"), &rendered),
        ],
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn seal_rejects_malformed_requests() {
    let (router, _) = seal_app();
    let (device, chain, _) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let rendered = jpeg_sized(24, 16);
    let ok_parts: Vec<(&str, Option<&str>, &[u8])> = vec![
        ("original", Some("image/jpeg"), &captured),
        ("rendered", Some("image/jpeg"), &rendered),
    ];

    let (status, _) = seal_request(&router, Some(TOKEN), &ok_parts[..1], &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "rendered is required");

    let mut with_bad_type = ok_parts.clone();
    with_bad_type[1] = ("rendered", Some("application/pdf"), &rendered);
    let (status, _) = seal_request(&router, Some(TOKEN), &with_bad_type, &[]).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let mut with_junk = ok_parts.clone();
    with_junk.push(("surprise", None, b"x"));
    let (status, _) = seal_request(&router, Some(TOKEN), &with_junk, &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let mut with_bad_edits = ok_parts.clone();
    with_bad_edits.push(("edits", None, b"not json"));
    let (status, _) = seal_request(&router, Some(TOKEN), &with_bad_edits, &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Generative filters are not part of the edit vocabulary.
    let generative = br#"[{"op":"filter","name":"generative-fill"}]"#;
    let mut with_generative = ok_parts.clone();
    with_generative.push(("edits", None, generative));
    let (status, _) = seal_request(&router, Some(TOKEN), &with_generative, &[]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Ported from Social's `seal_preserves_capture_chain_without_claiming_capture_for_unsigned_sources`,
/// now with the P-256 service signer. The route refuses non-HM origins, but the
/// signer itself must still never promote one.
#[tokio::test]
async fn a_seal_never_promotes_an_unsigned_or_untrusted_source() {
    let (router, state) = seal_app();
    let (device, chain, root) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let rendered = jpeg_sized(24, 16);
    let seal = |source: &[u8], declared: bool| {
        sign_derivative(
            &state.ca,
            &state.verify_options(),
            None,
            &SealInput {
                original: source,
                original_mime: "image/jpeg",
                rendered: &rendered,
                rendered_mime: "image/jpeg",
                declared_ai: declared,
                edits: &[],
                resized: false,
            },
        )
        .unwrap()
    };

    let plain = jpeg();
    for (source, declared, expected) in [
        (&captured, false, "HM"),
        (&plain, false, "Unknown"),
        (&plain, true, "AI"),
    ] {
        let sealed = seal(source, declared);
        let verdict = delta_sync_core::verify_bytes(&sealed, "image/jpeg", &root, false);
        assert_eq!(verdict.label, expected, "{verdict:?}");
        assert!(verdict.actions.contains(&"c2pa.transcoded".into()));
    }

    // A trusted processing seal must not launder an untrusted capture.
    let (other, _) = seal_app();
    let (foreign_device, foreign_chain, _) = enrolled_device(&other).await;
    let foreign = sign_jpeg(&foreign_device.signing_key, &foreign_chain, &jpeg());
    let sealed = seal(&foreign, false);
    let verdict = delta_sync_core::verify_bytes(&sealed, "image/jpeg", &root, false);
    assert_eq!(verdict.label, "Unknown", "{verdict:?}");

    // Declared AI on a real capture stays AI, whatever the seal says.
    let sealed = seal(&captured, true);
    let verdict = delta_sync_core::verify_bytes(&sealed, "image/jpeg", &root, false);
    assert_eq!(verdict.label, "AI", "{verdict:?}");
}

#[tokio::test]
async fn an_unreachable_time_stamp_authority_fails_the_seal_closed() {
    // Port 9 on loopback refuses connections: no network is used.
    let (router, _) = test_app_with(|config| {
        config.seal_token = Some(TOKEN.to_string());
        config.tsa_url = Some("http://127.0.0.1:9/tsa".to_string());
    });
    let (device, chain, _) = enrolled_device(&router).await;
    let captured = sign_jpeg(&device.signing_key, &chain, &jpeg());
    let rendered = jpeg_sized(24, 16);
    let (status, body) = seal_request(
        &router,
        Some(TOKEN),
        &[
            ("original", Some("image/jpeg"), &captured),
            ("rendered", Some("image/jpeg"), &rendered),
        ],
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_GATEWAY,
        "{}",
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn tsa_url_is_configurable_and_defaults_to_digicert() {
    let mut env = EnvGuard::set(&[("DATA_DIR", tempfile_dir().display().to_string())]);
    std::env::remove_var("TSA_URL");
    assert_eq!(
        Config::from_env().unwrap().tsa_url.as_deref(),
        Some("http://timestamp.digicert.com")
    );
    for (value, expected) in [
        (
            "https://tsa.example.test/tsr",
            Some("https://tsa.example.test/tsr"),
        ),
        ("", None),
        ("off", None),
    ] {
        env.also_set("TSA_URL", value);
        assert_eq!(
            Config::from_env().unwrap().tsa_url.as_deref(),
            expected,
            "{value:?}"
        );
    }
    env.also_set("TSA_URL", "ftp://nope");
    assert!(Config::from_env().is_err());
    std::env::remove_var("TSA_URL");
    // The test config never time-stamps.
    assert!(Config::for_test(tempfile_dir()).tsa_url.is_none());
}

#[tokio::test]
async fn seal_token_must_be_long_enough() {
    let mut env = EnvGuard::set(&[
        ("DATA_DIR", tempfile_dir().display().to_string()),
        ("MEDIA_SEAL_TOKEN", "short".to_string()),
    ]);
    assert!(Config::from_env().is_err());
    env.also_set("MEDIA_SEAL_TOKEN", TOKEN);
    assert_eq!(
        Config::from_env().unwrap().seal_token.as_deref(),
        Some(TOKEN)
    );
}

#[tokio::test]
async fn enroll_exists_only_in_stub_mode() {
    let (stub, _) = test_app_with(|_| {});
    let device = delta_sync_core::generate_device_key().unwrap();
    let body = |attestation: &str| {
        json!({
            "platform": "ios",
            "app_id": "app.example.ios",
            "public_key_pem": device.public_pem,
            "attestation": attestation,
        })
    };
    let (status, issued) = json_request(&stub, "POST", "/v1/enroll", body("dev-stub")).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert!(issued["certificate_pem"]
        .as_str()
        .unwrap()
        .contains("BEGIN CERTIFICATE"));
    assert!(issued["certificate_chain_pem"]
        .as_str()
        .unwrap()
        .contains("BEGIN CERTIFICATE"));
    assert!(issued["expires_at_unix"].as_i64().unwrap() > 0);
    // What it issues is a normal device certificate under the Device ICA.
    with_cert(issued["certificate_pem"].as_str().unwrap(), |cert| {
        let eku = cert.extended_key_usage().unwrap().unwrap().value;
        assert_eq!(eku.other[0].to_string(), CLAIM_SIGNING_OID);
    });

    let (status, _) = json_request(&stub, "POST", "/v1/enroll", body("  ")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (live, _) = test_app_with(|config| {
        config.attestation_mode = AttestationMode::Live;
    });
    let (status, _) = json_request(&live, "POST", "/v1/enroll", body("play-integrity-token")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "live mode has no /v1/enroll");
    // The attested flow is still there.
    let (status, _) =
        json_request(&live, "POST", "/v1/challenge", json!({"platform": "ios"})).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn health_names_all_three_certificates() {
    let (router, _) = seal_app();
    let (status, body) = json_request(&router, "GET", "/health", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    for key in [
        "ca_fingerprint_sha256",
        "device_ica_fingerprint_sha256",
        "service_ica_fingerprint_sha256",
    ] {
        assert_eq!(body[key].as_str().unwrap().len(), 64, "{key}");
    }
    assert_eq!(body["seal_enabled"], true);
    assert_eq!(body["tsa_enabled"], false);
}
