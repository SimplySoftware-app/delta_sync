use anyhow::{anyhow, bail};
use base64::{engine::general_purpose::STANDARD, Engine};
use ciborium::from_reader;
use serde::Deserialize;
use serde_bytes::ByteBuf;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use x509_parser::{certificate::X509Certificate, prelude::FromDer, time::ASN1Time};

pub const APPLE_APP_ATTEST_ROOT_PEM: &str =
    include_str!("../../certs/Apple_App_Attestation_Root_CA.pem");

const NONCE_OID: &str = "1.2.840.113635.100.8.2";

#[derive(Debug)]
pub struct AppAttestResult {
    pub environment: &'static str,
    pub app_id: String,
    pub key_id_b64: String,
}

#[derive(Deserialize)]
struct AttestationObject {
    fmt: String,
    #[serde(rename = "attStmt")]
    att_stmt: AttStmt,
    #[serde(rename = "authData")]
    auth_data: ByteBuf,
}

#[derive(Deserialize)]
struct AttStmt {
    x5c: Vec<ByteBuf>,
    #[allow(dead_code)]
    receipt: Option<ByteBuf>,
}

struct AuthParts {
    rp_id_hash: [u8; 32],
    counter: u32,
    aaguid: [u8; 16],
    credential_id: Vec<u8>,
}

/// Verifies an App Attest attestation object.
///
/// Real: CBOR structure, certificate chain to `trust_root_pem`, nonce, rpIdHash,
/// counter, AAGUID, and credential id.
/// Not done: exchanging the receipt with Apple's App Attest server.
pub fn verify(
    attestation_b64: &str,
    challenge: &str,
    uncompressed_point: &[u8],
    app_id: &str,
    key_id_b64: &str,
    trust_root_pem: &str,
) -> anyhow::Result<AppAttestResult> {
    let raw = STANDARD
        .decode(attestation_b64.trim())
        .map_err(|_| anyhow!("attestation is not standard base64"))?;
    let obj: AttestationObject =
        from_reader(raw.as_slice()).map_err(|err| anyhow!("attestation CBOR: {err}"))?;
    if obj.fmt != "apple-appattest" {
        bail!("attestation fmt is {}, expected apple-appattest", obj.fmt);
    }
    if obj.att_stmt.x5c.is_empty() {
        bail!("attestation is missing the x5c certificate chain");
    }

    let certs: Vec<X509Certificate<'_>> = obj
        .att_stmt
        .x5c
        .iter()
        .map(|der| {
            X509Certificate::from_der(der)
                .map(|(_, cert)| cert)
                .map_err(|err| anyhow!("x5c certificate: {err}"))
        })
        .collect::<anyhow::Result<_>>()?;
    verify_chain(&certs, trust_root_pem)?;

    let auth = parse_auth_data(&obj.auth_data)?;
    let client_data_hash = crate::challenge::client_data_hash(challenge, uncompressed_point);
    let mut nonce_input = Vec::with_capacity(auth_len(&obj.auth_data) + 32);
    nonce_input.extend_from_slice(&obj.auth_data);
    nonce_input.extend_from_slice(&client_data_hash);
    let expected_nonce: [u8; 32] = Sha256::digest(&nonce_input).into();
    let cert_nonce = nonce_from_leaf(&certs[0])?;
    if cert_nonce != expected_nonce {
        bail!("attestation nonce does not commit to this challenge and device key");
    }

    let rp_expected: [u8; 32] = Sha256::digest(app_id.as_bytes()).into();
    if auth.rp_id_hash != rp_expected {
        bail!("rpIdHash does not match app id {app_id}");
    }
    if auth.counter != 0 {
        bail!("attestation counter must be 0");
    }
    let environment = aaguid_environment(&auth.aaguid)?;

    let given_key_id = STANDARD
        .decode(key_id_b64.trim())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key_id_b64.trim()))
        .map_err(|_| anyhow!("key_id is not base64"))?;
    if given_key_id != auth.credential_id {
        bail!("key_id does not match the attested credential id");
    }

    Ok(AppAttestResult {
        environment,
        app_id: app_id.to_string(),
        key_id_b64: STANDARD.encode(&auth.credential_id),
    })
}

fn auth_len(data: &ByteBuf) -> usize {
    data.len()
}

fn parse_auth_data(data: &[u8]) -> anyhow::Result<AuthParts> {
    if data.len() < 55 {
        bail!("authData is too short");
    }
    let mut rp_id_hash = [0u8; 32];
    rp_id_hash.copy_from_slice(&data[0..32]);
    let flags = data[32];
    if flags & 0x40 == 0 {
        bail!("authData is missing attested credential data");
    }
    let counter = u32::from_be_bytes(data[33..37].try_into().unwrap());
    let mut aaguid = [0u8; 16];
    aaguid.copy_from_slice(&data[37..53]);
    let cred_len = u16::from_be_bytes(data[53..55].try_into().unwrap()) as usize;
    if data.len() < 55 + cred_len {
        bail!("authData credential id is truncated");
    }
    let credential_id = data[55..55 + cred_len].to_vec();
    Ok(AuthParts {
        rp_id_hash,
        counter,
        aaguid,
        credential_id,
    })
}

fn aaguid_environment(aaguid: &[u8; 16]) -> anyhow::Result<&'static str> {
    if aaguid == b"appattest\0\0\0\0\0\0\0" {
        Ok("production")
    } else if aaguid == b"appattestdevelop" {
        Ok("development")
    } else {
        bail!("unrecognized App Attest AAGUID")
    }
}

fn nonce_from_leaf(cert: &X509Certificate<'_>) -> anyhow::Result<[u8; 32]> {
    for ext in cert.extensions() {
        if ext.oid.to_id_string() == NONCE_OID {
            return extract_nonce(ext.value);
        }
    }
    bail!("credential certificate has no App Attest nonce extension")
}

fn extract_nonce(value: &[u8]) -> anyhow::Result<[u8; 32]> {
    // SEQUENCE { [1] EXPLICIT OCTET STRING (32) }
    if value.len() > 6 && value[0] == 0x30 {
        let mut i = 0;
        while i + 34 <= value.len() {
            if value[i] == 0x04 && value[i + 1] == 0x20 {
                let mut nonce = [0u8; 32];
                nonce.copy_from_slice(&value[i + 2..i + 34]);
                return Ok(nonce);
            }
            i += 1;
        }
    }
    bail!("could not read the nonce from the certificate extension")
}

fn verify_chain(certs: &[X509Certificate<'_>], trust_root_pem: &str) -> anyhow::Result<()> {
    if certs.is_empty() {
        bail!("empty certificate chain");
    }
    let root_der = crate::ca::pem_to_der(trust_root_pem, "CERTIFICATE")?;
    let (_, root) =
        X509Certificate::from_der(&root_der).map_err(|err| anyhow!("trust root: {err}"))?;
    let now = ASN1Time::from(OffsetDateTime::now_utc());
    for cert in certs {
        if !cert.validity().is_valid_at(now) {
            bail!("a certificate in the attestation chain is outside its validity period");
        }
    }

    for window in certs.windows(2) {
        window[0]
            .verify_signature(Some(window[1].public_key()))
            .map_err(|err| anyhow!("certificate chain signature: {err}"))?;
    }
    let last = certs.last().unwrap();
    let last_point = last.public_key().subject_public_key.data.as_ref();
    let root_point = root.public_key().subject_public_key.data.as_ref();
    if last_point != root_point {
        last.verify_signature(Some(root.public_key()))
            .map_err(|_| anyhow!("certificate chain does not end at the App Attest trust root"))?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn encode_nonce_extension(nonce: &[u8; 32]) -> Vec<u8> {
    let mut inner = vec![0x04, 0x20];
    inner.extend_from_slice(nonce);
    let mut tagged = vec![0xA1, inner.len() as u8];
    tagged.extend(inner);
    let mut seq = vec![0x30, tagged.len() as u8];
    seq.extend(tagged);
    seq
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::challenge::client_data_hash;
    use rcgen::{
        BasicConstraints, CertificateParams, CustomExtension, DistinguishedName, DnType, IsCa,
        KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256,
    };
    use time::{Duration, OffsetDateTime};

    #[test]
    fn live_verifier_accepts_a_chain_we_control() {
        let root_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let mid_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let leaf_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let now = OffsetDateTime::now_utc();

        let root_params = ca_params("Test App Attest Root", now);
        let root = root_params.self_signed(&root_key).unwrap();

        let mid_params = ca_params("Test App Attest Intermediate", now);
        let mid = mid_params.signed_by(&mid_key, &root, &root_key).unwrap();

        let app_id = "TESTTEAM.cloud.deltasync.capture";
        let challenge = "challenge-value";
        let device_point = vec![0x04u8; 65];
        let client_hash = client_data_hash(challenge, &device_point);
        let credential_id = vec![7u8; 32];

        let mut auth = Vec::new();
        auth.extend_from_slice(&Sha256::digest(app_id.as_bytes()));
        auth.push(0x40);
        auth.extend_from_slice(&0u32.to_be_bytes());
        auth.extend_from_slice(b"appattestdevelop");
        auth.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
        auth.extend_from_slice(&credential_id);

        let mut nonce_input = auth.clone();
        nonce_input.extend_from_slice(&client_hash);
        let nonce: [u8; 32] = Sha256::digest(&nonce_input).into();

        let mut leaf_params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "attested key");
        leaf_params.distinguished_name = dn;
        leaf_params.not_before = now - Duration::minutes(5);
        leaf_params.not_after = now + Duration::days(1);
        leaf_params.is_ca = IsCa::NoCa;
        leaf_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf_params.custom_extensions = vec![CustomExtension::from_oid_content(
            &[1, 2, 840, 113635, 100, 8, 2],
            encode_nonce_extension(&nonce),
        )];
        let leaf = leaf_params.signed_by(&leaf_key, &mid, &mid_key).unwrap();

        let attestation = attestation_cbor(&auth, &[leaf.der().to_vec(), mid.der().to_vec()]);
        let result = verify(
            &STANDARD.encode(&attestation),
            challenge,
            &device_point,
            app_id,
            &STANDARD.encode(&credential_id),
            &root.pem(),
        )
        .unwrap();
        assert_eq!(result.environment, "development");

        let apple_err = verify(
            &STANDARD.encode(&attestation),
            challenge,
            &device_point,
            app_id,
            &STANDARD.encode(&credential_id),
            APPLE_APP_ATTEST_ROOT_PEM,
        )
        .unwrap_err();
        assert!(apple_err.to_string().contains("trust root"), "{apple_err}");
    }

    fn ca_params(cn: &str, now: OffsetDateTime) -> CertificateParams {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, cn);
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        params.not_before = now - Duration::minutes(5);
        params.not_after = now + Duration::days(30);
        params
    }

    fn attestation_cbor(auth: &[u8], x5c: &[Vec<u8>]) -> Vec<u8> {
        let mut encoder = Vec::new();
        let obj = AttestationForEncode {
            fmt: "apple-appattest",
            att_stmt: AttStmtEncode {
                x5c: x5c.iter().map(|c| ByteBuf::from(c.clone())).collect(),
                receipt: ByteBuf::new(),
            },
            auth_data: ByteBuf::from(auth.to_vec()),
        };
        ciborium::into_writer(&obj, &mut encoder).unwrap();
        encoder
    }

    #[derive(serde::Serialize)]
    struct AttestationForEncode<'a> {
        fmt: &'a str,
        #[serde(rename = "attStmt")]
        att_stmt: AttStmtEncode,
        #[serde(rename = "authData")]
        auth_data: ByteBuf,
    }

    #[derive(serde::Serialize)]
    struct AttStmtEncode {
        x5c: Vec<ByteBuf>,
        receipt: ByteBuf,
    }
}
