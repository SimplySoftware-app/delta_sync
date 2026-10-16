//! Dev certificate authority.
//!
//! The device generates the key. The authority only sees the public key, and
//! only after attestation (stubbed in dev by the HTTP service). Production
//! replaces this in-memory CA. Nothing here is a C2PA trust-list certificate.

use std::sync::Arc;

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose, SubjectPublicKeyInfo, PKCS_ED25519,
};
use time::{Duration, OffsetDateTime};

use crate::error::CoreError;

#[derive(Clone, Debug)]
pub struct DeviceKey {
    pub private_pem: String,
    pub public_pem: String,
}

pub struct IssuedCert {
    pub certificate_pem: String,
    pub ca_pem: String,
    pub expires_at_unix: i64,
}

struct Inner {
    ca_key: KeyPair,
    ca_cert: Certificate,
    ca_pem: String,
}

#[derive(Clone)]
pub struct DevAuthority {
    inner: Arc<Inner>,
}

impl DevAuthority {
    /// Reload a private development CA so restarts do not orphan old credentials.
    pub fn from_pem(key_pem: &str, ca_pem: &str) -> Result<Self, CoreError> {
        let ca_key = KeyPair::from_pem(key_pem).map_err(|e| CoreError::msg(e.to_string()))?;
        let params = CertificateParams::from_ca_cert_pem(ca_pem)
            .map_err(|e| CoreError::msg(e.to_string()))?;
        let ca_cert = params
            .self_signed(&ca_key)
            .map_err(|e| CoreError::msg(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Inner {
                ca_key,
                ca_cert,
                ca_pem: ca_pem.to_owned(),
            }),
        })
    }

    pub fn private_pem(&self) -> String {
        self.inner.ca_key.serialize_pem()
    }

    pub fn generate() -> Result<Self, CoreError> {
        let ca_key =
            KeyPair::generate_for(&PKCS_ED25519).map_err(|e| CoreError::msg(e.to_string()))?;
        let mut params = CertificateParams::new(vec!["delta-sync-dev-ca.local".into()])
            .map_err(|e| CoreError::msg(e.to_string()))?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "Delta Sync Dev CA");
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let ca_cert = params
            .self_signed(&ca_key)
            .map_err(|e| CoreError::msg(e.to_string()))?;
        let ca_pem = ca_cert.pem();
        Ok(Self {
            inner: Arc::new(Inner {
                ca_key,
                ca_cert,
                ca_pem,
            }),
        })
    }

    pub fn ca_pem(&self) -> &str {
        &self.inner.ca_pem
    }

    /// Issue a 7-day end-entity certificate for a device public key.
    pub fn issue(&self, public_key_pem: &str, device_name: &str) -> Result<IssuedCert, CoreError> {
        let public_key = SubjectPublicKeyInfo::from_pem(public_key_pem)
            .map_err(|e| CoreError::msg(format!("public key: {e}")))?;
        let name = if device_name.trim().is_empty() {
            "device"
        } else {
            device_name.trim()
        };
        let mut params = CertificateParams::new(vec![format!("{name}.delta-sync.local")])
            .map_err(|e| CoreError::msg(e.to_string()))?;
        params
            .distinguished_name
            .push(DnType::CommonName, "Delta Sync device");
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        // c2pa rejects an end-entity certificate that has no extended key usage
        // and no authority key identifier. Email protection is one EKU the
        // 0.91 profile accepts without a custom trust list.
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::EmailProtection];
        params.use_authority_key_identifier_extension = true;
        let now = OffsetDateTime::now_utc();
        let expires = now + Duration::days(7);
        params.not_before = now;
        params.not_after = expires;
        let cert = params
            .signed_by(&public_key, &self.inner.ca_cert, &self.inner.ca_key)
            .map_err(|e| CoreError::msg(e.to_string()))?;
        Ok(IssuedCert {
            certificate_pem: cert.pem(),
            ca_pem: self.inner.ca_pem.clone(),
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
    }
}
