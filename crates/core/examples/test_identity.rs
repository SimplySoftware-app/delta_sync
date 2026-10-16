//! Print a throwaway signing identity as JSON for the Swift and Kotlin tests.
//!
//! The platform tests implement `DeviceSigner` with a software P-256 key (the
//! Secure Enclave and StrongBox are not available to them). This example
//! generates that key, a certificate chain issued by a fresh dev CA, and the
//! CA, at test time. Nothing is committed and nothing outlives the test run.
//!
//! Output: `{"ca_pem", "chain_pem", "pkcs8_der_base64", "x963_private_base64"}`.
//! The last one is the ANSI X9.63 private key (`04 || X || Y || D`) that
//! `SecKeyCreateWithData` takes on Apple platforms.

use base64::Engine;
use delta_sync_core::DevAuthority;
use p256::ecdsa::SigningKey;
use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;

fn main() {
    let authority = DevAuthority::generate().expect("dev CA");
    let key = SigningKey::random(&mut OsRng);
    let public_pem = key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let issued = authority.issue(&public_pem, "platform-test").unwrap();
    let der = key.to_pkcs8_der().unwrap();
    let mut x963 = key
        .verifying_key()
        .to_encoded_point(false)
        .as_bytes()
        .to_vec();
    x963.extend_from_slice(&key.to_bytes());
    let b64 = base64::engine::general_purpose::STANDARD;
    println!(
        "{}",
        serde_json::json!({
            "ca_pem": issued.ca_pem,
            "chain_pem": format!("{}\n{}", issued.certificate_pem, issued.ca_pem),
            "pkcs8_der_base64": b64.encode(der.as_bytes()),
            "x963_private_base64": b64.encode(&x963),
        })
    );
}
