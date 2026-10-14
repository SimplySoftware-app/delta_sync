use std::{fs, path::Path};

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand_core::{OsRng, RngCore};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CertificateSigningRequestParams,
    DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
    SerialNumber, PKCS_ECDSA_P256_SHA256,
};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use x509_parser::{
    certificate::X509Certificate, certification_request::X509CertificationRequest, prelude::FromDer,
};
use zeroize::Zeroizing;

use crate::challenge::Platform;

pub struct CertificateAuthority {
    cert_pem: String,
    key_pem: Zeroizing<String>,
    fingerprint_sha256: String,
}

pub struct IssuedCert {
    pub certificate_pem: String,
    pub certificate_chain_pem: String,
    pub expires_at: OffsetDateTime,
    pub serial_hex: String,
}

impl CertificateAuthority {
    pub fn load_or_create(
        data_dir: &Path,
        cert_override: Option<&Path>,
        key_override: Option<&Path>,
    ) -> anyhow::Result<Self> {
        fs::create_dir_all(data_dir).with_context(|| format!("creating {}", data_dir.display()))?;

        let (cert_pem, key_pem) = if let (Some(cert_path), Some(key_path)) =
            (cert_override, key_override)
        {
            let cert_pem = fs::read_to_string(cert_path)
                .with_context(|| format!("reading {}", cert_path.display()))?;
            let key_pem = fs::read_to_string(key_path)
                .with_context(|| format!("reading {}", key_path.display()))?;
            (cert_pem, key_pem)
        } else {
            let cert_path = data_dir.join("ca.crt.pem");
            let key_path = data_dir.join("ca.key.pem");
            if cert_path.is_file() && key_path.is_file() {
                (
                    fs::read_to_string(&cert_path)?,
                    fs::read_to_string(&key_path)?,
                )
            } else if cert_path.exists() || key_path.exists() {
                anyhow::bail!(
                    "CA files are incomplete in {}. Refusing to overwrite a partial key.",
                    data_dir.display()
                );
            } else {
                let (cert_pem, key_pem) = generate_ca()?;
                write_private(&key_path, key_pem.as_bytes())?;
                fs::write(&cert_path, cert_pem.as_bytes())?;
                tracing::warn!(
                    path = %cert_path.display(),
                    "generated a new DeltaSync CA. This key is not in git. Back it up before the data volume is destroyed."
                );
                (cert_pem, key_pem)
            }
        };

        let fingerprint_sha256 = cert_fingerprint(&cert_pem)?;
        Ok(Self {
            cert_pem,
            key_pem: Zeroizing::new(key_pem),
            fingerprint_sha256,
        })
    }

    pub fn certificate_pem(&self) -> &str {
        &self.cert_pem
    }

    pub fn fingerprint_sha256(&self) -> &str {
        &self.fingerprint_sha256
    }

    pub fn issue(
        &self,
        csr_pem: &str,
        platform: Platform,
        app_id: &str,
        ttl: std::time::Duration,
    ) -> anyhow::Result<IssuedCert> {
        verify_csr_and_point(csr_pem)?;
        let issuer_key = KeyPair::from_pem(&self.key_pem)?;
        let issuer = issuer_handle(&self.cert_pem, &issuer_key)?;

        let mut csr = CertificateSigningRequestParams::from_pem(csr_pem)
            .map_err(|err| anyhow!("could not parse CSR: {err}"))?;
        let now = OffsetDateTime::now_utc();
        let expires_at = now + Duration::seconds(i64::try_from(ttl.as_secs()).unwrap_or(900));
        csr.params.not_before = now - Duration::minutes(5);
        csr.params.not_after = expires_at;
        csr.params.is_ca = IsCa::NoCa;
        csr.params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        csr.params.extended_key_usages = vec![ExtendedKeyUsagePurpose::EmailProtection];
        csr.params.use_authority_key_identifier_extension = true;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "DeltaSync device");
        dn.push(
            DnType::OrganizationalUnitName,
            format!("{}:{app_id}", platform.as_str()),
        );
        csr.params.distinguished_name = dn;
        let mut serial_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut serial_bytes);
        serial_bytes[0] &= 0x7f;
        csr.params.serial_number = Some(SerialNumber::from_slice(&serial_bytes));

        let certificate = csr
            .signed_by(&issuer, &issuer_key)
            .map_err(|err| anyhow!("signing device certificate: {err}"))?;
        let certificate_pem = certificate.pem();
        let certificate_chain_pem = format!("{certificate_pem}{}", self.cert_pem);
        Ok(IssuedCert {
            certificate_pem,
            certificate_chain_pem,
            expires_at,
            serial_hex: hex::encode(serial_bytes),
        })
    }
}

fn generate_ca() -> anyhow::Result<(String, String)> {
    let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "DeltaSync Signing Authority");
    dn.push(DnType::OrganizationName, "DeltaSync");
    params.distinguished_name = dn;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::minutes(5);
    params.not_after = now + Duration::days(3650);
    let cert = params.self_signed(&key)?;
    Ok((cert.pem(), key.serialize_pem()))
}

/// rcgen cannot reload an issued certificate, but signing only needs the issuer
/// name and the same private key. Rebuild a throwaway issuer with the subject
/// from the stored CA certificate after checking the key matches.
fn issuer_handle(cert_pem: &str, key: &KeyPair) -> anyhow::Result<Certificate> {
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("CA certificate: {err}"))?;
    let cert_point = parsed.public_key().subject_public_key.data.as_ref();
    if cert_point != key.public_key_raw() {
        anyhow::bail!("CA private key does not match CA certificate");
    }
    let mut params = CertificateParams::default();
    params.distinguished_name = dn_from_parsed(&parsed)?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(1);
    params.not_after = now + Duration::days(1);
    params
        .self_signed(key)
        .map_err(|err| anyhow!("rebuilding CA issuer handle: {err}"))
}

fn dn_from_parsed(cert: &X509Certificate<'_>) -> anyhow::Result<DistinguishedName> {
    let mut dn = DistinguishedName::new();
    for rdn in cert.subject().iter_rdn() {
        for attr in rdn.iter() {
            let value = attr
                .as_str()
                .map_err(|_| anyhow!("CA subject attribute is not a string"))?;
            let kind = match attr.attr_type().to_id_string().as_str() {
                "2.5.4.3" => Some(DnType::CommonName),
                "2.5.4.10" => Some(DnType::OrganizationName),
                "2.5.4.11" => Some(DnType::OrganizationalUnitName),
                "2.5.4.6" => Some(DnType::CountryName),
                _ => None,
            };
            if let Some(kind) = kind {
                dn.push(kind, value);
            }
        }
    }
    Ok(dn)
}

fn cert_fingerprint(cert_pem: &str) -> anyhow::Result<String> {
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let digest = Sha256::digest(&der);
    Ok(hex::encode(digest))
}

pub fn verify_csr_and_point(csr_pem: &str) -> anyhow::Result<Vec<u8>> {
    let der = pem_to_der(csr_pem, "CERTIFICATE REQUEST")?;
    let (_, csr) = X509CertificationRequest::from_der(&der)
        .map_err(|err| anyhow!("CSR DER is not valid: {err}"))?;
    csr.verify_signature()
        .map_err(|err| anyhow!("CSR signature is not valid: {err}"))?;
    let point = csr
        .certification_request_info
        .subject_pki
        .subject_public_key
        .data
        .to_vec();
    if point.len() != 65 || point[0] != 0x04 {
        anyhow::bail!("device key must be an uncompressed P-256 point");
    }
    Ok(point)
}

pub(crate) fn pem_to_der(pem: &str, label: &str) -> anyhow::Result<Vec<u8>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = pem.find(&begin).ok_or_else(|| anyhow!("missing {begin}"))? + begin.len();
    let stop = pem[start..]
        .find(&end)
        .ok_or_else(|| anyhow!("missing {end}"))?
        + start;
    let b64: String = pem[start..stop]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    STANDARD.decode(b64).context("PEM base64")
}

fn write_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(bytes)?;
    Ok(())
}

trait OpenOptionsExt {
    fn mode(&mut self, mode: u32) -> &mut Self;
}

impl OpenOptionsExt for fs::OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut Self {
        std::os::unix::fs::OpenOptionsExt::mode(self, mode);
        self
    }
}
