//! A time-stamp authority for tests. It answers RFC 3161 requests in memory,
//! with a throwaway root and TSA certificate made at run time, so no test ever
//! reaches the network. The time it stamps is chosen by the test.

use delta_sync_core::{CoreError, TimestampProvider};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::pkcs8::EncodePrivateKey;
use rand_core::OsRng;
use rcgen::{
    BasicConstraints, CertificateParams, CustomExtension, DistinguishedName, DnType, IsCa, KeyPair,
    KeyUsagePurpose, PKCS_ECDSA_P256_SHA256,
};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

pub struct MockTsa {
    key: SigningKey,
    leaf_der: Vec<u8>,
    issuer_der: Vec<u8>,
    serial_der: Vec<u8>,
    /// The root the verifier must be told to trust for time stamps.
    pub root_pem: String,
    /// The time every token says.
    pub stamped_at: OffsetDateTime,
}

impl MockTsa {
    pub fn new(stamped_at: OffsetDateTime) -> Self {
        let root_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let mut root_params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Mock TSA Root");
        root_params.distinguished_name = dn;
        root_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        root_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        root_params.not_before = OffsetDateTime::now_utc() - Duration::days(30);
        root_params.not_after = OffsetDateTime::now_utc() + Duration::days(30);
        let root = root_params.self_signed(&root_key).unwrap();

        let key = SigningKey::random(&mut OsRng);
        let leaf_pem = key.to_pkcs8_pem(p256::pkcs8::LineEnding::LF).unwrap();
        let leaf_key = KeyPair::from_pem(leaf_pem.as_str()).unwrap();
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Mock TSA");
        params.distinguished_name = dn;
        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        // id-kp-timeStamping, the only EKU, marked critical as RFC 3161 asks.
        let mut eku = CustomExtension::from_oid_content(
            &[2, 5, 29, 37],
            der(0x30, &[der(0x06, &[0x2B, 6, 1, 5, 5, 7, 3, 8])].concat()),
        );
        eku.set_criticality(true);
        params.custom_extensions = vec![eku];
        params.use_authority_key_identifier_extension = true;
        params.not_before = OffsetDateTime::now_utc() - Duration::days(30);
        params.not_after = OffsetDateTime::now_utc() + Duration::days(30);
        let serial = [0x01u8, 0x42, 0x17];
        params.serial_number = Some(rcgen::SerialNumber::from_slice(&serial));
        let leaf = params.signed_by(&leaf_key, &root, &root_key).unwrap();

        // The issuer name as it is written in the certificate.
        let (_, parsed) = x509_parser::parse_x509_certificate(leaf.der()).unwrap();
        Self {
            key,
            leaf_der: leaf.der().to_vec(),
            issuer_der: parsed.issuer().as_raw().to_vec(),
            serial_der: der(0x02, &serial),
            root_pem: root.pem(),
            stamped_at,
        }
    }

    /// Answer a DER `TimeStampReq` with a DER `TimeStampResp`.
    pub fn respond(&self, request: &[u8]) -> Vec<u8> {
        let (imprint, nonce) = parse_request(request);
        let gen_time = self.stamped_at.format(&time::macros::format_description!(
            "[year][month][day][hour][minute][second]Z"
        ));
        let mut tst = vec![
            der(0x02, &[1]),
            der(0x06, &[0x2A, 3, 4, 1]), // a made-up policy: 1.2.3.4.1
            imprint,
            der(0x02, &[0x2A]),
            der(0x18, gen_time.unwrap().as_bytes()),
        ];
        if let Some(nonce) = nonce {
            tst.push(nonce);
        }
        let tst_info = der(0x30, &tst.concat());

        let sha256 = der(0x30, &[der(0x06, &SHA256_OID), der(0x05, &[])].concat());
        let attrs = [
            attribute(&CONTENT_TYPE_OID, der(0x06, &TST_INFO_OID)),
            attribute(&MESSAGE_DIGEST_OID, der(0x04, &Sha256::digest(&tst_info))),
        ];
        // DER sorts the members of a SET OF by their encoding. The verifier
        // re-encodes the attributes, so they have to be signed in that order.
        let mut sorted: Vec<Vec<u8>> = attrs.to_vec();
        sorted.sort();
        let attrs_content = sorted.concat();
        let signature: Signature = self.key.sign(&der(0x31, &attrs_content));

        let signer_info = der(
            0x30,
            &[
                der(0x02, &[1]),
                der(
                    0x30,
                    &[self.issuer_der.clone(), self.serial_der.clone()].concat(),
                ),
                sha256.clone(),
                der(0xA0, &attrs_content),
                der(0x30, &der(0x06, &ECDSA_SHA256_OID)),
                der(0x04, signature.to_der().as_bytes()),
            ]
            .concat(),
        );
        let signed_data = der(
            0x30,
            &[
                der(0x02, &[3]),
                der(0x31, &sha256),
                der(
                    0x30,
                    &[der(0x06, &TST_INFO_OID), der(0xA0, &der(0x04, &tst_info))].concat(),
                ),
                der(0xA0, &self.leaf_der),
                der(0x31, &signer_info),
            ]
            .concat(),
        );
        let token = der(
            0x30,
            &[der(0x06, &SIGNED_DATA_OID), der(0xA0, &signed_data)].concat(),
        );
        der(0x30, &[der(0x30, &der(0x02, &[0])), token].concat())
    }
}

impl TimestampProvider for MockTsa {
    fn timestamp(&self, request: Vec<u8>) -> Result<Vec<u8>, CoreError> {
        Ok(self.respond(&request))
    }
}

const SHA256_OID: [u8; 9] = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const ECDSA_SHA256_OID: [u8; 8] = [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
const SIGNED_DATA_OID: [u8; 9] = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];
const TST_INFO_OID: [u8; 11] = [
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x01, 0x04,
];
const CONTENT_TYPE_OID: [u8; 9] = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x03];
const MESSAGE_DIGEST_OID: [u8; 9] = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04];

fn attribute(oid: &[u8], value: Vec<u8>) -> Vec<u8> {
    der(0x30, &[der(0x06, oid), der(0x31, &value)].concat())
}

fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    match content.len() {
        n if n < 0x80 => out.push(n as u8),
        n if n < 0x100 => out.extend([0x81, n as u8]),
        n => out.extend([0x82, (n >> 8) as u8, n as u8]),
    }
    out.extend_from_slice(content);
    out
}

/// Read one TLV: (tag, content, rest).
fn read(data: &[u8]) -> (u8, &[u8], &[u8]) {
    let tag = data[0];
    let (len, header) = match data[1] {
        n if n < 0x80 => (n as usize, 2),
        0x81 => (data[2] as usize, 3),
        0x82 => (((data[2] as usize) << 8) | data[3] as usize, 4),
        other => panic!("unsupported DER length {other:#x}"),
    };
    (tag, &data[header..header + len], &data[header + len..])
}

/// The message imprint (whole TLV) and the nonce (whole TLV) of a `TimeStampReq`.
fn parse_request(request: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    let (_, body, _) = read(request);
    let (_, _, rest) = read(body); // version
    let (tag, imprint_content, rest) = read(rest);
    assert_eq!(tag, 0x30, "message imprint");
    let imprint = der(0x30, imprint_content);
    let nonce = (!rest.is_empty() && rest[0] == 0x02).then(|| {
        let (_, content, _) = read(rest);
        der(0x02, content)
    });
    (imprint, nonce)
}
