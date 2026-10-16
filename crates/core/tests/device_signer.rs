//! A `DeviceSigner` implemented in Rust stands in for the Secure Enclave and
//! Android Keystore: the private key lives in the signer and only signatures
//! cross the boundary. Real hardware keys cannot be tested off-device.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use delta_sync_core::{
    sign_capture_file, verify_file, CoreError, DevAuthority, DeviceSigner, SignerAlg,
};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::pkcs8::{EncodePublicKey, LineEnding};
use rand_core::OsRng;

struct RustSigner {
    key: SigningKey,
    chain: String,
    calls: AtomicUsize,
    der: bool,
}

impl RustSigner {
    fn new(authority: &DevAuthority) -> Arc<Self> {
        Self::build(authority, false)
    }

    fn build(authority: &DevAuthority, der: bool) -> Arc<Self> {
        let key = SigningKey::random(&mut OsRng);
        let public_pem = key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        let issued = authority.issue(&public_pem, "rust-test-signer").unwrap();
        Arc::new(Self {
            key,
            chain: format!("{}\n{}", issued.certificate_pem, issued.ca_pem),
            calls: AtomicUsize::new(0),
            der,
        })
    }
}

impl DeviceSigner for RustSigner {
    fn alg(&self) -> SignerAlg {
        SignerAlg::Es256
    }

    fn cert_chain(&self) -> Result<String, CoreError> {
        Ok(self.chain.clone())
    }

    fn sign(&self, data: Vec<u8>) -> Result<Vec<u8>, CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let signature: Signature = self.key.sign(&data);
        Ok(if self.der {
            signature.to_der().as_bytes().to_vec()
        } else {
            signature.to_bytes().to_vec()
        })
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "delta-sync-core-{name}-{}-{:x}",
            std::process::id(),
            rand_suffix()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rand_suffix() -> u64 {
    use rand_core::RngCore;
    OsRng.next_u64()
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn sign_and_verify(name: &str) {
    let authority = DevAuthority::generate().unwrap();
    let signer = RustSigner::new(&authority);
    let scratch = Scratch::new("ok");
    let out = scratch.path(name);
    sign_capture_file(&fixture(name), &out, signer.clone()).unwrap();
    assert!(signer.calls.load(Ordering::SeqCst) >= 1);
    assert!(fs::metadata(&out).unwrap().len() > fs::metadata(fixture(name)).unwrap().len());

    let verdict = verify_file(&out, None, authority.ca_pem(), false);
    assert_eq!(verdict.label, "HM", "{name}: {verdict:?}");
    assert!(verdict.trusted, "{name}: {verdict:?}");

    // The same file against another CA is not trusted.
    let other = DevAuthority::generate().unwrap();
    let verdict = verify_file(&out, None, other.ca_pem(), false);
    assert_eq!(verdict.label, "Unknown", "{name}: {verdict:?}");

    // An unsigned original is Unknown.
    let verdict = verify_file(&fixture(name), None, authority.ca_pem(), false);
    assert_eq!(verdict.label, "Unknown", "{name}: {verdict:?}");
}

#[test]
fn signs_a_jpeg_and_verifies_hm() {
    sign_and_verify("sample.jpg");
}

#[test]
fn signs_a_mov_and_verifies_hm() {
    sign_and_verify("sample.mov");
}

#[test]
fn signs_an_mp4_and_verifies_hm() {
    sign_and_verify("sample.mp4");
}

#[test]
fn a_changed_byte_makes_the_capture_unknown() {
    for name in ["sample.jpg", "sample.mov", "sample.mp4"] {
        let authority = DevAuthority::generate().unwrap();
        let scratch = Scratch::new("tamper");
        let out = scratch.path(name);
        sign_capture_file(&fixture(name), &out, RustSigner::new(&authority)).unwrap();
        let mut bytes = fs::read(&out).unwrap();
        // The manifest sits near the front, so a byte a quarter of the original
        // size from the end is media payload that the hash covers.
        let original = fs::read(fixture(name)).unwrap();
        let at = bytes.len() - original.len() / 4;
        bytes[at] ^= 0xff;
        let tampered = scratch.path(&format!("tampered-{name}"));
        fs::write(&tampered, bytes).unwrap();
        let verdict = verify_file(&tampered, None, authority.ca_pem(), false);
        assert_ne!(verdict.label, "HM", "{name}: {verdict:?}");
    }
}

#[test]
fn der_signatures_are_rejected_with_a_clear_error() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("der");
    let out = scratch.path("out.jpg");
    let err = sign_capture_file(
        &fixture("sample.jpg"),
        &out,
        RustSigner::build(&authority, true),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("raw r||s"), "{err}");
    assert!(!out.exists(), "a failed signing must not leave an output");
}

#[test]
fn rejects_unsupported_extensions_and_in_place_signing() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("bad");
    let png = scratch.path("a.png");
    fs::write(&png, b"not really").unwrap();
    let err = sign_capture_file(&png, &scratch.path("b.png"), RustSigner::new(&authority))
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported file extension"), "{err}");

    let copy = scratch.path("c.jpg");
    fs::copy(fixture("sample.jpg"), &copy).unwrap();
    let err = sign_capture_file(&copy, &copy, RustSigner::new(&authority))
        .unwrap_err()
        .to_string();
    assert!(err.contains("must differ"), "{err}");
    assert!(copy.exists(), "the input must survive");
}

#[test]
fn a_failing_signer_surfaces_its_error() {
    struct Broken;
    impl DeviceSigner for Broken {
        fn alg(&self) -> SignerAlg {
            SignerAlg::Es256
        }
        fn cert_chain(&self) -> Result<String, CoreError> {
            Err(CoreError::msg("key was invalidated by a biometry change"))
        }
        fn sign(&self, _data: Vec<u8>) -> Result<Vec<u8>, CoreError> {
            unreachable!()
        }
    }
    let scratch = Scratch::new("broken");
    let err = sign_capture_file(
        &fixture("sample.mp4"),
        &scratch.path("o.mp4"),
        Arc::new(Broken),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("biometry"), "{err}");
}
