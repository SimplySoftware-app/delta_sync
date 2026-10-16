//! Delta Sync PKI.
//!
//! ```text
//! Root (P-256, offline)
//! ├── Device ICA   issues device certificates (24 h at most), only after attestation
//! └── Service ICA  issues the long-lived certificate the service seals derivatives with
//! ```
//!
//! Both kinds of leaf carry the C2PA claim-signing EKU (1.3.6.1.4.1.62558.2.1),
//! so a verifier can tell a device capture from a service seal by the ICA the
//! chain goes through. A compromised device cannot issue seals and the service
//! cannot fabricate captures.
//!
//! The root key is never written to disk by this service. On a first boot with an
//! empty `DATA_DIR` the service generates a root, signs the two ICAs with it and
//! drops the root key. Production should provision the root and the ICAs from
//! outside (see `PkiPaths`).

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand_core::{OsRng, RngCore};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyIdMethod, KeyPair, KeyUsagePurpose, PublicKeyData,
    SerialNumber, SubjectPublicKeyInfo, PKCS_ECDSA_P256_SHA256,
};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use x509_parser::{
    certificate::X509Certificate, certification_request::X509CertificationRequest,
    extensions::ParsedExtension, prelude::FromDer,
};
use zeroize::Zeroizing;

use crate::{
    challenge::Platform,
    config::{PkiPaths, MAX_DEVICE_CERT_TTL_SECONDS},
};

/// `c2pa-kp-claimSigning`.
pub const CLAIM_SIGNING_EKU: [u64; 9] = [1, 3, 6, 1, 4, 1, 62558, 2, 1];
/// The same OID in dotted form, for the C2PA verifier's EKU list.
pub const CLAIM_SIGNING_EKU_OID: &str = "1.3.6.1.4.1.62558.2.1";

const ROOT_YEARS_DAYS: i64 = 3650;
const ICA_DAYS: i64 = 1825;
const SERVICE_CERT_DAYS: i64 = 365;
/// A generated service certificate is replaced when it has less life than this.
const SERVICE_RENEW_BEFORE_DAYS: i64 = 30;

const ROOT_CN: &str = "Delta Sync Root CA";
// The label rules in the core read a manifest's role from these names.
use delta_sync_core::{DEVICE_ICA_CN, SERVICE_ICA_CN};
const ORG: &str = "Delta Sync";
/// Subject organisation of device leaves. C2PA reports it as the signature issuer.
pub const DEVICE_LEAF_ORG: &str = "Delta Sync Device";
/// Subject organisation of the service leaf.
pub const SERVICE_LEAF_ORG: &str = "Delta Sync Service";

struct Ica {
    cert_pem: String,
    key_pem: Zeroizing<String>,
    fingerprint_sha256: String,
}

pub struct CertificateAuthority {
    root_pem: String,
    root_fingerprint: String,
    device_ica: Ica,
    service_ica: Ica,
    /// Service leaf followed by the Service ICA: what a manifest carries.
    service_chain_pem: String,
    service_key_pem: Zeroizing<String>,
    service_fingerprint: String,
}

pub struct IssuedCert {
    pub certificate_pem: String,
    /// Leaf followed by the Device ICA. Root is the client's trust anchor.
    pub certificate_chain_pem: String,
    pub expires_at: OffsetDateTime,
    pub serial_hex: String,
}

impl CertificateAuthority {
    pub fn load_or_create(data_dir: &Path, paths: &PkiPaths) -> anyhow::Result<Self> {
        fs::create_dir_all(data_dir).with_context(|| format!("creating {}", data_dir.display()))?;

        let (root_pem, device_ica, service_ica) = load_hierarchy(data_dir, paths)?;
        let (service_cert_pem, service_key_pem) =
            load_service_credentials(data_dir, paths, &service_ica)?;
        let service_chain_pem = format!(
            "{}{}",
            ensure_trailing_newline(&service_cert_pem),
            service_ica.cert_pem
        );
        let service_fingerprint = cert_fingerprint(&service_cert_pem)?;
        let root_fingerprint = cert_fingerprint(&root_pem)?;
        Ok(Self {
            root_pem,
            root_fingerprint,
            device_ica,
            service_ica,
            service_chain_pem,
            service_key_pem: Zeroizing::new(service_key_pem),
            service_fingerprint,
        })
    }

    /// The trust anchor clients and verifiers use.
    pub fn root_pem(&self) -> &str {
        &self.root_pem
    }

    pub fn device_ica_pem(&self) -> &str {
        &self.device_ica.cert_pem
    }

    pub fn service_ica_pem(&self) -> &str {
        &self.service_ica.cert_pem
    }

    /// Fingerprint of the root. Kept under the old name for `/health` readers.
    pub fn fingerprint_sha256(&self) -> &str {
        &self.root_fingerprint
    }

    pub fn device_ica_fingerprint_sha256(&self) -> &str {
        &self.device_ica.fingerprint_sha256
    }

    pub fn service_ica_fingerprint_sha256(&self) -> &str {
        &self.service_ica.fingerprint_sha256
    }

    pub fn service_fingerprint_sha256(&self) -> &str {
        &self.service_fingerprint
    }

    /// Service leaf + Service ICA, PEM. The chain a derivative manifest carries.
    pub fn service_chain_pem(&self) -> &str {
        &self.service_chain_pem
    }

    /// The service signing key, PKCS#8 PEM. It is loaded from disk, never made per request.
    pub fn service_key_pem(&self) -> &str {
        &self.service_key_pem
    }

    /// Issue a device certificate for the key in a CSR whose signature and key
    /// type were already checked by [`verify_csr_and_point`].
    pub fn issue(
        &self,
        csr_pem: &str,
        platform: Platform,
        app_id: &str,
        ttl: std::time::Duration,
    ) -> anyhow::Result<IssuedCert> {
        // Only the public key is taken from the CSR. Its subject and any
        // extensions it requests are ignored: the authority decides them.
        let public_key = csr_public_key(csr_pem)?;
        self.issue_device_leaf(&public_key, platform.as_str(), app_id, ttl)
    }

    /// Issue a device certificate for a bare public key (SubjectPublicKeyInfo PEM).
    /// Only the stub-mode `/v1/enroll` migration route uses this.
    pub fn issue_for_public_key(
        &self,
        public_key_pem: &str,
        platform: &str,
        app_id: &str,
        ttl: std::time::Duration,
    ) -> anyhow::Result<IssuedCert> {
        let public_key = SubjectPublicKeyInfo::from_pem(public_key_pem)
            .map_err(|err| anyhow!("public key: {err}"))?;
        self.issue_device_leaf(&public_key, platform, app_id, ttl)
    }

    fn issue_device_leaf(
        &self,
        public_key: &impl PublicKeyData,
        platform: &str,
        app_id: &str,
        ttl: std::time::Duration,
    ) -> anyhow::Result<IssuedCert> {
        let ttl_secs = ttl.as_secs().min(MAX_DEVICE_CERT_TTL_SECONDS);
        let ttl_secs = i64::try_from(ttl_secs).unwrap_or(900);
        let issuer_key = KeyPair::from_pem(&self.device_ica.key_pem)?;
        let issuer = issuer_handle(&self.device_ica.cert_pem, &issuer_key)?;

        let mut params = leaf_params(
            "Delta Sync device",
            DEVICE_LEAF_ORG,
            Some(format!("{platform}:{app_id}")),
        );
        let now = OffsetDateTime::now_utc();
        let expires_at = now + Duration::seconds(ttl_secs);
        params.not_before = now - Duration::minutes(5);
        params.not_after = expires_at;
        let mut serial_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut serial_bytes);
        serial_bytes[0] &= 0x7f;
        params.serial_number = Some(SerialNumber::from_slice(&serial_bytes));

        let certificate = params
            .signed_by(public_key, &issuer, &issuer_key)
            .map_err(|err| anyhow!("signing device certificate: {err}"))?;
        let certificate_pem = certificate.pem();
        let certificate_chain_pem = format!("{certificate_pem}{}", self.device_ica.cert_pem);
        Ok(IssuedCert {
            certificate_pem,
            certificate_chain_pem,
            expires_at,
            serial_hex: hex::encode(serial_bytes),
        })
    }
}

fn leaf_params(cn: &str, org: &str, ou: Option<String>) -> CertificateParams {
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::Other(CLAIM_SIGNING_EKU.to_vec())];
    params.use_authority_key_identifier_extension = true;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, cn);
    dn.push(DnType::OrganizationName, org);
    if let Some(ou) = ou {
        dn.push(DnType::OrganizationalUnitName, ou);
    }
    params.distinguished_name = dn;
    params
}

fn ca_params(cn: &str, path_len: u8, days: i64, with_eku: bool) -> CertificateParams {
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, cn);
    dn.push(DnType::OrganizationName, ORG);
    params.distinguished_name = dn;
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(path_len));
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    if with_eku {
        params.extended_key_usages =
            vec![ExtendedKeyUsagePurpose::Other(CLAIM_SIGNING_EKU.to_vec())];
    }
    params.use_authority_key_identifier_extension = true;
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::minutes(5);
    params.not_after = now + Duration::days(days);
    params
}

struct Generated {
    root_pem: String,
    device_ica: (String, String),
    service_ica: (String, String),
}

/// Make a fresh root and sign both ICAs. The root key is dropped here.
fn generate_hierarchy() -> anyhow::Result<Generated> {
    let root_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let root = ca_params(ROOT_CN, 1, ROOT_YEARS_DAYS, false)
        .self_signed(&root_key)
        .map_err(|err| anyhow!("root certificate: {err}"))?;
    let sign_ica = |cn: &str| -> anyhow::Result<(String, String)> {
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
        let cert = ca_params(cn, 0, ICA_DAYS, true)
            .signed_by(&key, &root, &root_key)
            .map_err(|err| anyhow!("{cn}: {err}"))?;
        Ok((cert.pem(), key.serialize_pem()))
    };
    let device_ica = sign_ica(DEVICE_ICA_CN)?;
    let service_ica = sign_ica(SERVICE_ICA_CN)?;
    Ok(Generated {
        root_pem: root.pem(),
        device_ica,
        service_ica,
    })
}

fn load_hierarchy(data_dir: &Path, paths: &PkiPaths) -> anyhow::Result<(String, Ica, Ica)> {
    let provisioned = [
        &paths.root_cert,
        &paths.device_ica_cert,
        &paths.device_ica_key,
        &paths.service_ica_cert,
        &paths.service_ica_key,
    ];
    let given = provisioned.iter().filter(|p| p.is_some()).count();
    let (root_pem, device, service) = if given == provisioned.len() {
        (
            read(paths.root_cert.as_deref().unwrap())?,
            (
                read(paths.device_ica_cert.as_deref().unwrap())?,
                read(paths.device_ica_key.as_deref().unwrap())?,
            ),
            (
                read(paths.service_ica_cert.as_deref().unwrap())?,
                read(paths.service_ica_key.as_deref().unwrap())?,
            ),
        )
    } else if given > 0 {
        bail!(
            "ROOT_CERT_PATH, DEVICE_ICA_CERT_PATH, DEVICE_ICA_KEY_PATH, SERVICE_ICA_CERT_PATH and SERVICE_ICA_KEY_PATH must be set together"
        );
    } else {
        let files = [
            data_dir.join("root.crt.pem"),
            data_dir.join("device-ica.crt.pem"),
            data_dir.join("device-ica.key.pem"),
            data_dir.join("service-ica.crt.pem"),
            data_dir.join("service-ica.key.pem"),
        ];
        let present = files.iter().filter(|p| p.exists()).count();
        if present == files.len() {
            (
                read(&files[0])?,
                (read(&files[1])?, read(&files[2])?),
                (read(&files[3])?, read(&files[4])?),
            )
        } else if present > 0 {
            bail!(
                "PKI files are incomplete in {}. Refusing to overwrite a partial hierarchy.",
                data_dir.display()
            );
        } else {
            let generated = generate_hierarchy()?;
            write_private(&files[2], generated.device_ica.1.as_bytes())?;
            write_private(&files[4], generated.service_ica.1.as_bytes())?;
            fs::write(&files[1], generated.device_ica.0.as_bytes())?;
            fs::write(&files[3], generated.service_ica.0.as_bytes())?;
            fs::write(&files[0], generated.root_pem.as_bytes())?;
            tracing::warn!(
                dir = %data_dir.display(),
                "generated a new Delta Sync PKI (root, Device ICA, Service ICA). The root key was not stored. Back up the ICA keys, or provision the PKI from outside before a public host."
            );
            (
                generated.root_pem,
                generated.device_ica,
                generated.service_ica,
            )
        }
    };

    let device_ica = into_ica(device, &root_pem, "Device ICA")?;
    let service_ica = into_ica(service, &root_pem, "Service ICA")?;
    Ok((root_pem, device_ica, service_ica))
}

fn into_ica(parts: (String, String), root_pem: &str, name: &str) -> anyhow::Result<Ica> {
    let (cert_pem, key_pem) = parts;
    let key = KeyPair::from_pem(&key_pem).with_context(|| format!("{name} key"))?;
    ensure_key_matches(&cert_pem, &key).with_context(|| name.to_string())?;
    ensure_issued_by(&cert_pem, root_pem)
        .with_context(|| format!("{name} is not signed by the root"))?;
    ensure_is_ca(&cert_pem).with_context(|| name.to_string())?;
    Ok(Ica {
        fingerprint_sha256: cert_fingerprint(&cert_pem)?,
        cert_pem,
        key_pem: Zeroizing::new(key_pem),
    })
}

/// The service leaf and key. They are read from disk (or the configured paths)
/// on every boot. A key is only generated when none exists, so a restart never
/// changes the signer.
fn load_service_credentials(
    data_dir: &Path,
    paths: &PkiPaths,
    service_ica: &Ica,
) -> anyhow::Result<(String, String)> {
    match (&paths.service_cert, &paths.service_key) {
        (Some(cert), Some(key)) => {
            let cert_pem = first_certificate(&read(cert)?)?;
            let key_pem = read(key)?;
            check_service_pair(&cert_pem, &key_pem, service_ica)?;
            if days_left(&cert_pem)? < SERVICE_RENEW_BEFORE_DAYS {
                tracing::warn!("the configured service certificate expires in under 30 days");
            }
            return Ok((cert_pem, key_pem));
        }
        (None, None) => {}
        _ => bail!("SERVICE_CERT_PATH and SERVICE_KEY_PATH must be set together"),
    }

    let cert_path = data_dir.join("service.crt.pem");
    let key_path = data_dir.join("service.key.pem");
    match (cert_path.exists(), key_path.exists()) {
        (true, true) => {
            let cert_pem = read(&cert_path)?;
            let key_pem = read(&key_path)?;
            let healthy = check_service_pair(&cert_pem, &key_pem, service_ica).is_ok()
                && days_left(&cert_pem)? >= SERVICE_RENEW_BEFORE_DAYS;
            if healthy {
                return Ok((cert_pem, key_pem));
            }
            tracing::warn!("the stored service certificate is expiring or does not match the Service ICA; issuing a new one");
            let renewed = issue_service_leaf(service_ica)?;
            replace_private(&key_path, renewed.1.as_bytes())?;
            replace_public(&cert_path, renewed.0.as_bytes())?;
            Ok(renewed)
        }
        (false, false) => {
            let issued = issue_service_leaf(service_ica)?;
            write_private(&key_path, issued.1.as_bytes())?;
            fs::write(&cert_path, issued.0.as_bytes())?;
            tracing::info!(path = %cert_path.display(), "issued the service signing certificate");
            Ok(issued)
        }
        _ => bail!(
            "service certificate files are incomplete in {}. Refusing to overwrite a partial key.",
            data_dir.display()
        ),
    }
}

fn issue_service_leaf(service_ica: &Ica) -> anyhow::Result<(String, String)> {
    let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let issuer_key = KeyPair::from_pem(&service_ica.key_pem)?;
    let issuer = issuer_handle(&service_ica.cert_pem, &issuer_key)?;
    let mut params = leaf_params("Delta Sync service", SERVICE_LEAF_ORG, None);
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::minutes(5);
    params.not_after = now + Duration::days(SERVICE_CERT_DAYS);
    let mut serial = [0u8; 16];
    OsRng.fill_bytes(&mut serial);
    serial[0] &= 0x7f;
    params.serial_number = Some(SerialNumber::from_slice(&serial));
    let cert = params
        .signed_by(&key, &issuer, &issuer_key)
        .map_err(|err| anyhow!("signing service certificate: {err}"))?;
    Ok((cert.pem(), key.serialize_pem()))
}

fn check_service_pair(cert_pem: &str, key_pem: &str, service_ica: &Ica) -> anyhow::Result<()> {
    let key = KeyPair::from_pem(key_pem).context("service key")?;
    ensure_key_matches(cert_pem, &key).context("service certificate")?;
    ensure_issued_by(cert_pem, &service_ica.cert_pem)
        .context("service certificate is not signed by the Service ICA")
}

fn days_left(cert_pem: &str) -> anyhow::Result<i64> {
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("certificate: {err}"))?;
    let not_after = parsed.validity().not_after.timestamp();
    Ok((not_after - OffsetDateTime::now_utc().unix_timestamp()) / 86_400)
}

/// rcgen cannot reload an issued certificate, but signing only needs the issuer
/// name, its key usages and the same private key. Rebuild a throwaway issuer from
/// the stored certificate after checking the key matches. The subject key
/// identifier is carried over so a leaf's authority key identifier points at the
/// real ICA even when that ICA was made by other tooling.
fn issuer_handle(cert_pem: &str, key: &KeyPair) -> anyhow::Result<Certificate> {
    ensure_key_matches(cert_pem, key)?;
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("CA certificate: {err}"))?;
    let mut params = CertificateParams::default();
    params.distinguished_name = dn_from_parsed(&parsed)?;
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    for extension in parsed.extensions() {
        if let ParsedExtension::SubjectKeyIdentifier(id) = extension.parsed_extension() {
            params.key_identifier_method = KeyIdMethod::PreSpecified(id.0.to_vec());
        }
    }
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

fn ensure_key_matches(cert_pem: &str, key: &KeyPair) -> anyhow::Result<()> {
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("certificate: {err}"))?;
    let cert_point = parsed.public_key().subject_public_key.data.as_ref();
    if cert_point != key.public_key_raw() {
        bail!("private key does not match the certificate");
    }
    Ok(())
}

fn ensure_issued_by(cert_pem: &str, issuer_pem: &str) -> anyhow::Result<()> {
    let child_der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let issuer_der = pem_to_der(issuer_pem, "CERTIFICATE")?;
    let (_, child) =
        X509Certificate::from_der(&child_der).map_err(|err| anyhow!("certificate: {err}"))?;
    let (_, issuer) =
        X509Certificate::from_der(&issuer_der).map_err(|err| anyhow!("issuer: {err}"))?;
    if child.issuer() != issuer.subject() {
        bail!("issuer name does not match");
    }
    child
        .verify_signature(Some(issuer.public_key()))
        .map_err(|err| anyhow!("signature does not verify: {err}"))
}

fn ensure_is_ca(cert_pem: &str) -> anyhow::Result<()> {
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|err| anyhow!("certificate: {err}"))?;
    if !parsed.is_ca() {
        bail!("certificate is not a CA");
    }
    Ok(())
}

fn cert_fingerprint(cert_pem: &str) -> anyhow::Result<String> {
    let der = pem_to_der(cert_pem, "CERTIFICATE")?;
    let digest = Sha256::digest(&der);
    Ok(hex::encode(digest))
}

/// The CSR's public key after its signature and key type were checked.
fn csr_public_key(csr_pem: &str) -> anyhow::Result<SubjectPublicKeyInfo> {
    verify_csr_and_point(csr_pem)?;
    let der = pem_to_der(csr_pem, "CERTIFICATE REQUEST")?;
    let (_, csr) = X509CertificationRequest::from_der(&der)
        .map_err(|err| anyhow!("CSR DER is not valid: {err}"))?;
    SubjectPublicKeyInfo::from_der(csr.certification_request_info.subject_pki.raw)
        .map_err(|err| anyhow!("CSR public key: {err}"))
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

/// The first CERTIFICATE block of a PEM bundle, as its own PEM.
fn first_certificate(pem: &str) -> anyhow::Result<String> {
    let der = pem_to_der(pem, "CERTIFICATE")?;
    let b64 = STANDARD.encode(der);
    let mut out = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk)?);
        out.push('\n');
    }
    out.push_str("-----END CERTIFICATE-----\n");
    Ok(out)
}

fn ensure_trailing_newline(pem: &str) -> String {
    if pem.ends_with('\n') {
        pem.to_string()
    } else {
        format!("{pem}\n")
    }
}

fn read(path: &Path) -> anyhow::Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn write_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let mut file = private_options()
        .create_new(true)
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Replace a private file without ever exposing a partial write or looser mode.
fn replace_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let tmp: PathBuf = path.with_extension("pem.tmp");
    let _ = fs::remove_file(&tmp);
    let mut file = private_options()
        .create_new(true)
        .open(&tmp)
        .with_context(|| format!("creating {}", tmp.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

fn replace_public(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp: PathBuf = path.with_extension("pem.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

fn private_options() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}
