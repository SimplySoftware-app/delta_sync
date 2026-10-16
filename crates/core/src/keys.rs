//! Dev certificate authority.
//!
//! The same shape as the production PKI in `server/src/ca.rs`:
//!
//! ```text
//! Root
//! ├── Device ICA   issues device certificates
//! └── Service ICA  issues the certificate the service seals derivatives with
//! ```
//!
//! Every certificate below the root carries the C2PA claim-signing EKU
//! (`c2pa-kp-claimSigning`, 1.3.6.1.4.1.62558.2.1). The label rules tell a device
//! capture from a service seal by the ICA a manifest's chain goes through.
//!
//! The device generates its key. The authority only sees the public key, and only
//! after attestation (stubbed in dev by the HTTP service). This in-memory
//! authority is for tests and examples. Nothing here is a C2PA trust-list
//! certificate.

use std::sync::Arc;

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SubjectPublicKeyInfo,
    PKCS_ECDSA_P256_SHA256, PKCS_ED25519,
};
use time::{Duration, OffsetDateTime};

use crate::error::CoreError;

/// `c2pa-kp-claimSigning`.
const CLAIM_SIGNING_EKU: [u64; 9] = [1, 3, 6, 1, 4, 1, 62558, 2, 1];

/// Subject common names of the two ICAs. The label rules read the role of a
/// manifest from the ICA in its certificate chain, so these names are part of
/// the trust model: only the root can sign a certificate that carries them.
pub const DEVICE_ICA_CN: &str = "Delta Sync Device ICA";
pub const SERVICE_ICA_CN: &str = "Delta Sync Service ICA";

const ORG: &str = "Delta Sync";
/// Subject organisation of device leaves. C2PA reports it as the signature issuer.
pub const DEVICE_LEAF_ORG: &str = "Delta Sync Device";
/// Subject organisation of the service leaf.
pub const SERVICE_LEAF_ORG: &str = "Delta Sync Service";

/// Default life of a device certificate from [`DevAuthority::issue`].
const DEVICE_CERT_MINUTES: i64 = 15;
const SERVICE_CERT_DAYS: i64 = 365;

#[derive(Clone, Debug)]
pub struct DeviceKey {
    pub private_pem: String,
    pub public_pem: String,
}

pub struct IssuedCert {
    /// The leaf certificate followed by its ICA: the chain a manifest carries.
    pub certificate_pem: String,
    /// The leaf certificate alone.
    pub leaf_pem: String,
    /// The root, the trust anchor of a verifier.
    pub ca_pem: String,
    pub expires_at_unix: i64,
}

struct Ica {
    key: KeyPair,
    cert: Certificate,
    pem: String,
}

struct Inner {
    root_pem: String,
    device_ica: Ica,
    service_ica: Ica,
}

#[derive(Clone)]
pub struct DevAuthority {
    inner: Arc<Inner>,
}

fn ca_params(cn: &str, path_len: u8, with_eku: bool) -> CertificateParams {
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
    params.not_after = now + Duration::days(3650);
    params
}

fn leaf_params(cn: &str, org: &str) -> CertificateParams {
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::Other(CLAIM_SIGNING_EKU.to_vec())];
    params.use_authority_key_identifier_extension = true;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, cn);
    dn.push(DnType::OrganizationName, org);
    params.distinguished_name = dn;
    params
}

impl DevAuthority {
    pub fn generate() -> Result<Self, CoreError> {
        let err = |e: rcgen::Error| CoreError::msg(e.to_string());
        let root_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).map_err(err)?;
        let root = ca_params("Delta Sync Dev Root CA", 1, false)
            .self_signed(&root_key)
            .map_err(err)?;
        let make_ica = |cn: &str| -> Result<Ica, CoreError> {
            let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).map_err(err)?;
            let cert = ca_params(cn, 0, true)
                .signed_by(&key, &root, &root_key)
                .map_err(err)?;
            let pem = cert.pem();
            Ok(Ica { key, cert, pem })
        };
        let device_ica = make_ica(DEVICE_ICA_CN)?;
        let service_ica = make_ica(SERVICE_ICA_CN)?;
        Ok(Self {
            inner: Arc::new(Inner {
                root_pem: root.pem(),
                device_ica,
                service_ica,
            }),
        })
    }

    /// The root: the trust anchor a verifier is given.
    pub fn ca_pem(&self) -> &str {
        &self.inner.root_pem
    }

    pub fn device_ica_pem(&self) -> &str {
        &self.inner.device_ica.pem
    }

    pub fn service_ica_pem(&self) -> &str {
        &self.inner.service_ica.pem
    }

    /// Issue a short-lived device certificate (15 minutes) under the Device ICA.
    pub fn issue(&self, public_key_pem: &str, device_name: &str) -> Result<IssuedCert, CoreError> {
        let now = OffsetDateTime::now_utc();
        self.issue_device_between(
            public_key_pem,
            device_name,
            now - Duration::minutes(5),
            now + Duration::minutes(DEVICE_CERT_MINUTES),
        )
    }

    /// Issue a device certificate valid for an exact window. Tests use it to
    /// make a certificate that has already expired.
    pub fn issue_device_between(
        &self,
        public_key_pem: &str,
        device_name: &str,
        not_before: OffsetDateTime,
        not_after: OffsetDateTime,
    ) -> Result<IssuedCert, CoreError> {
        let name = if device_name.trim().is_empty() {
            "device"
        } else {
            device_name.trim()
        };
        let mut params = leaf_params(&format!("Delta Sync device {name}"), DEVICE_LEAF_ORG);
        params.not_before = not_before;
        params.not_after = not_after;
        self.sign_leaf(&self.inner.device_ica, params, public_key_pem)
    }

    /// Issue the service's sealing certificate (one year) under the Service ICA.
    pub fn issue_service(
        &self,
        public_key_pem: &str,
        service_name: &str,
    ) -> Result<IssuedCert, CoreError> {
        let now = OffsetDateTime::now_utc();
        let mut params = leaf_params(
            &format!("Delta Sync service {}", service_name.trim()),
            SERVICE_LEAF_ORG,
        );
        params.not_before = now - Duration::minutes(5);
        params.not_after = now + Duration::days(SERVICE_CERT_DAYS);
        self.sign_leaf(&self.inner.service_ica, params, public_key_pem)
    }

    fn sign_leaf(
        &self,
        ica: &Ica,
        params: CertificateParams,
        public_key_pem: &str,
    ) -> Result<IssuedCert, CoreError> {
        let public_key = SubjectPublicKeyInfo::from_pem(public_key_pem)
            .map_err(|e| CoreError::msg(format!("public key: {e}")))?;
        let expires = params.not_after;
        let cert = params
            .signed_by(&public_key, &ica.cert, &ica.key)
            .map_err(|e| CoreError::msg(e.to_string()))?;
        let leaf_pem = cert.pem();
        Ok(IssuedCert {
            certificate_pem: format!("{leaf_pem}{}", ica.pem),
            leaf_pem,
            ca_pem: self.inner.root_pem.clone(),
            expires_at_unix: expires.unix_timestamp(),
        })
    }
}

pub fn generate_device_key() -> Result<DeviceKey, CoreError> {
    let key = KeyPair::generate_for(&PKCS_ED25519).map_err(|e| CoreError::msg(e.to_string()))?;
    Ok(DeviceKey {
        private_pem: key.serialize_pem(),
        public_pem: key.public_key_pem(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_round_trips_a_public_key() {
        let authority = DevAuthority::generate().unwrap();
        let device = generate_device_key().unwrap();
        let issued = authority.issue(&device.public_pem, "pixel").unwrap();
        assert!(issued.certificate_pem.contains("BEGIN CERTIFICATE"));
        assert!(issued.ca_pem.contains("BEGIN CERTIFICATE"));
        assert!(issued.expires_at_unix > 0);
        let service = authority.issue_service(&device.public_pem, "seal").unwrap();
        assert!(service.certificate_pem.contains(&service.leaf_pem));
    }
}
