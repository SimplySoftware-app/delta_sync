//! Sign a JPEG/MOV/MP4 with a throwaway software key and print the verdict.
//! Used to measure memory on large files: `sign_file <in> <out>`.

use std::env;
use std::path::Path;
use std::sync::Arc;

use delta_sync_core::{
    sign_capture_file, verify_file, CoreError, DevAuthority, DeviceSigner, SignerAlg,
};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::pkcs8::{EncodePublicKey, LineEnding};
use rand_core::OsRng;

struct Software {
    key: SigningKey,
    chain: String,
}

impl DeviceSigner for Software {
    fn alg(&self) -> SignerAlg {
        SignerAlg::Es256
    }
    fn cert_chain(&self) -> Result<String, CoreError> {
        Ok(self.chain.clone())
    }
    fn sign(&self, data: Vec<u8>) -> Result<Vec<u8>, CoreError> {
        let sig: Signature = self.key.sign(&data);
        Ok(sig.to_bytes().to_vec())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (input, output) = (&args[1], &args[2]);
    let authority = DevAuthority::generate().expect("dev CA");
    let key = SigningKey::random(&mut OsRng);
    let pem = key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let issued = authority.issue(&pem, "example").unwrap();
    let signer = Arc::new(Software {
        key,
        chain: format!("{}\n{}", issued.certificate_pem, issued.ca_pem),
    });
    sign_capture_file(Path::new(input), Path::new(output), signer).expect("sign");
    if args.get(3).map(String::as_str) == Some("--no-verify") {
        return;
    }
    let verdict = verify_file(Path::new(output), None, authority.ca_pem(), false);
    println!("{} {}", verdict.label, verdict.summary);
}
