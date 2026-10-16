//! Deterministic acceptance of EXTERNAL cameras via the official C2PA lists.
//!
//! An external capture (a manifest NOT signed by our own device authority) is
//! HM only if every condition below holds. Otherwise the verdict is
//! `Unknown`. Nothing is guessed.
//!
//! 1. The signer certificate chains to the pinned C2PA Trust List anchors.
//! 2. The signer certificate subject `O` and `CN` match a conformance record
//!    with `productType == generatorProduct` and `status == conformant`.
//! 3. That record's `maxAssuranceLevel` is at least [`MIN_EXTERNAL_ASSURANCE`].
//! 4. The claim generator version is at least the record's `minVersion` (when
//!    set) and the asset MIME type is in the record's `containers.generate`.
//! 5. The manifest has `c2pa.created` with `digitalSourceType` digitalCapture
//!    and no action, source type or assertion named in `disallowedSignals`.
//! 6. (Label integration, later) Subsequent edits must be permitted actions
//!    or our own service seal. Not decided here.
//!
//! The lists are vendored under `trust/c2pa/` and embedded at compile time, so
//! the verdict of a given binary is fixed by its build. `SOURCE.lock` pins the
//! upstream commit and the sha256 of each file; a test fails if they drift.
//!
//! Not checked here: certificate revocation (CRL/OCSP) and the TSA list. The
//! signing time comes from the caller: pass the trusted timestamp time when
//! the manifest has one, otherwise the current time is used.
//!
//! Signature algorithms are those `x509-parser` verifies through `ring`
//! (RSA PKCS#1, ECDSA P-256/P-384 with SHA-256/384, Ed25519). A chain that
//! needs anything else (for instance RSA-PSS) fails closed as `Unknown`.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use x509_parser::certificate::X509Certificate;
use x509_parser::der_parser::oid::Oid;
use x509_parser::oid_registry::{OID_PKCS1_SHA1WITHRSA, OID_SHA1_WITH_RSA};
use x509_parser::pem::Pem;
use x509_parser::prelude::FromDer;
use x509_parser::time::ASN1Time;

use crate::DIGITAL_CAPTURE;

/// Minimum `maxAssuranceLevel` a conformance record needs. Changing it is a
/// code change on purpose: it is not read from configuration at runtime.
pub const MIN_EXTERNAL_ASSURANCE: u8 = 2;

static TRUST_LIST_PEM: &[u8] = include_bytes!("../../../trust/c2pa/C2PA-TRUST-LIST.pem");
static PRODUCTS_JSON: &[u8] = include_bytes!("../../../trust/c2pa/conforming-products-list.json");

/// One action of the manifest, as the caller extracted it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalAction {
    /// For example `c2pa.created`.
    pub action: String,
    /// IPTC `digitalSourceType` URL, when the action carries one.
    pub digital_source_type: Option<String>,
}

/// Everything the verdict needs from an external manifest.
#[derive(Clone, Debug, Default)]
pub struct ExternalManifestInfo {
    /// Signer certificate chain in DER, leaf first.
    pub cert_chain_der: Vec<Vec<u8>>,
    /// Trusted timestamp time (unix seconds). `None` uses the current time.
    pub signing_time_unix: Option<i64>,
    pub claim_generator_name: String,
    pub claim_generator_version: Option<String>,
    pub mime: String,
    pub actions: Vec<ExternalAction>,
    /// Labels of the other assertions in the manifest.
    pub assertion_labels: Vec<String>,
}

/// The conformance record that accepted the capture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalProduct {
    pub organization: String,
    pub common_name: String,
    pub min_version: Option<String>,
    pub assurance_level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalVerdict {
    Hm { product: ExternalProduct },
    Unknown { reason: String },
}

#[derive(Debug, Deserialize)]
struct Record {
    status: String,
    product: Product,
    #[serde(default)]
    containers: Containers,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Product {
    product_type: String,
    #[serde(rename = "DN")]
    dn: Dn,
    #[serde(default)]
    min_version: Option<String>,
    #[serde(default)]
    assurance: Option<Assurance>,
    #[serde(default)]
    disallowed_signals: Option<DisallowedSignals>,
}

#[derive(Debug, Deserialize)]
struct Dn {
    #[serde(rename = "CN", default)]
    cn: String,
    #[serde(rename = "O", default)]
    o: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Assurance {
    #[serde(default)]
    max_assurance_level: Option<u8>,
}

#[derive(Debug, Default, Deserialize)]
struct DisallowedSignals {
    #[serde(default)]
    inception: Vec<String>,
    #[serde(default)]
    transformation: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Containers {
    #[serde(default)]
    generate: std::collections::BTreeMap<String, Vec<String>>,
}

impl Record {
    fn level(&self) -> u8 {
        self.product
            .assurance
            .as_ref()
            .and_then(|a| a.max_assurance_level)
            .unwrap_or(0)
    }

    fn disallowed(&self) -> impl Iterator<Item = &str> {
        self.product
            .disallowed_signals
            .iter()
            .flat_map(|d| d.inception.iter().chain(d.transformation.iter()))
            .map(String::as_str)
    }
}

/// Anchors plus conformance records. Production code only uses
/// [`TrustStore::vendored`]; tests inject their own through `new`.
pub struct TrustStore {
    anchors: Vec<Vec<u8>>,
    records: Vec<Record>,
}

impl TrustStore {
    /// `anchors_der`: trust anchors in DER. `products_json`: conformance list.
    pub(crate) fn new(anchors_der: Vec<Vec<u8>>, products_json: &[u8]) -> Result<Self, String> {
        let records: Vec<Record> =
            serde_json::from_slice(products_json).map_err(|e| format!("conformance list: {e}"))?;
        Ok(Self {
            anchors: anchors_der,
            records,
        })
    }

    fn from_pem(trust_pem: &[u8], products_json: &[u8]) -> Result<Self, String> {
        let mut anchors = Vec::new();
        for pem in Pem::iter_from_buffer(trust_pem) {
            let pem = pem.map_err(|e| format!("trust list PEM: {e}"))?;
            if pem.label == "CERTIFICATE" {
                anchors.push(pem.contents);
            }
        }
        if anchors.is_empty() {
            return Err("trust list has no certificates".into());
        }
        Self::new(anchors, products_json)
    }

    /// The store built from the vendored official lists.
    pub fn vendored() -> Result<&'static TrustStore, String> {
        static STORE: OnceLock<Result<TrustStore, String>> = OnceLock::new();
        STORE
            .get_or_init(|| TrustStore::from_pem(TRUST_LIST_PEM, PRODUCTS_JSON))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Products (`O`, `CN`) that can produce an HM capture under rules 2, 3
    /// and 5: generator, conformant, assurance high enough, and not barred
    /// from claiming a digital capture.
    pub fn eligible_products(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .records
            .iter()
            .filter(|r| {
                r.status == "conformant"
                    && r.product.product_type == "generatorProduct"
                    && r.level() >= MIN_EXTERNAL_ASSURANCE
                    && !r.disallowed().any(signal_matches_capture)
            })
            .map(|r| (r.product.dn.o.clone(), r.product.dn.cn.clone()))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn verdict_at(&self, info: &ExternalManifestInfo, at_unix: i64) -> ExternalVerdict {
        match self.evaluate(info, at_unix) {
            Ok(product) => ExternalVerdict::Hm { product },
            Err(reason) => ExternalVerdict::Unknown { reason },
        }
    }

    fn evaluate(
        &self,
        info: &ExternalManifestInfo,
        at_unix: i64,
    ) -> Result<ExternalProduct, String> {
        // 1. Chain to the pinned anchors.
        let at = info.signing_time_unix.unwrap_or(at_unix);
        let (org, cn) = self.validate_chain(&info.cert_chain_der, at)?;

        // 2-5, evaluated per record: the same DN can appear in several
        // versions. Any record that passes everything accepts the capture;
        // otherwise report the failure that got furthest.
        let candidates: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| r.product.dn.o == org && r.product.dn.cn == cn)
            .collect();
        if candidates.is_empty() {
            return Err(format!(
                "signer {org} / {cn} is not in the C2PA conforming products list"
            ));
        }
        let mut worst: Option<(u8, String)> = None;
        for record in candidates {
            match check_record(record, info) {
                Ok(product) => return Ok(product),
                Err((stage, msg)) => {
                    if worst.as_ref().is_none_or(|(s, _)| stage > *s) {
                        worst = Some((stage, msg));
                    }
                }
            }
        }
        Err(worst.map(|(_, m)| m).unwrap_or_default())
    }

    /// Returns the leaf subject `(O, CN)` when the chain is trusted at `at`.
    fn validate_chain(&self, chain: &[Vec<u8>], at: i64) -> Result<(String, String), String> {
        if chain.is_empty() {
            return Err("manifest has no signer certificate".into());
        }
        let at_time =
            ASN1Time::from_timestamp(at).map_err(|_| "invalid signing time".to_string())?;
        let certs: Vec<X509Certificate> = chain
            .iter()
            .map(|der| match X509Certificate::from_der(der) {
                Ok(([], cert)) => Ok(cert),
                _ => Err("signer certificate chain is not valid DER".to_string()),
            })
            .collect::<Result<_, _>>()?;
        let anchors: Vec<X509Certificate> = self
            .anchors
            .iter()
            .filter_map(|der| X509Certificate::from_der(der).ok().map(|(_, c)| c))
            .collect();

        let leaf = &certs[0];
        if leaf.is_ca() {
            return Err("signer certificate is a CA certificate".into());
        }
        let trusted = (|| {
            for (i, cert) in certs.iter().enumerate() {
                if !cert.validity().is_valid_at(at_time) {
                    return Err(format!(
                        "certificate {i} of the chain is not valid at the signing time"
                    ));
                }
                if i > 0 && !cert.is_ca() {
                    return Err(format!("certificate {i} of the chain is not a CA"));
                }
                if uses_sha1(cert) {
                    return Err(format!("certificate {i} of the chain is signed with SHA-1"));
                }
                // The chain itself reached a trust anchor.
                if self
                    .anchors
                    .iter()
                    .any(|a| a.as_slice() == chain[i].as_slice())
                {
                    return Ok(());
                }
                if anchors.iter().any(|a| {
                    a.is_ca()
                        && a.validity().is_valid_at(at_time)
                        && a.subject().as_raw() == cert.issuer().as_raw()
                        && cert.verify_signature(Some(a.public_key())).is_ok()
                }) {
                    return Ok(());
                }
                match certs.get(i + 1) {
                    Some(next)
                        if next.subject().as_raw() == cert.issuer().as_raw()
                            && cert.verify_signature(Some(next.public_key())).is_ok() => {}
                    _ => {
                        return Err(
                            "signer certificate does not chain to the C2PA Trust List".into()
                        )
                    }
                }
            }
            Err("signer certificate does not chain to the C2PA Trust List".into())
        })();
        trusted?;

        let org = first_attr(leaf.subject().iter_organization().next());
        let cn = first_attr(leaf.subject().iter_common_name().next());
        match (org, cn) {
            (Some(o), Some(c)) => Ok((o, c)),
            _ => Err("signer certificate subject lacks O or CN".into()),
        }
    }
}

fn first_attr(attr: Option<&x509_parser::x509::AttributeTypeAndValue>) -> Option<String> {
    attr.and_then(|a| a.as_str().ok()).map(str::to_owned)
}

fn uses_sha1(cert: &X509Certificate) -> bool {
    let oid: &Oid = &cert.signature_algorithm.algorithm;
    *oid == OID_PKCS1_SHA1WITHRSA || *oid == OID_SHA1_WITH_RSA
}

/// Conformance `inception` names map to IPTC source types by their last path
/// segment. Only the ones we can meet are listed.
fn iptc_equivalent(signal: &str) -> Option<&'static str> {
    match signal {
        "capturedMedia" => Some("digitalCapture"),
        "nonGenAIDigitalCreation" => Some("digitalCreation"),
        _ => None,
    }
}

fn signal_matches_capture(signal: &str) -> bool {
    signal == "capturedMedia" || signal == "digitalCapture"
}

fn last_segment(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}

/// Rules 2 (record kind), 3, 4 and 5 against one record. The error carries a
/// stage so the caller can report the failure that got furthest.
fn check_record(
    record: &Record,
    info: &ExternalManifestInfo,
) -> Result<ExternalProduct, (u8, String)> {
    let name = format!("{} / {}", record.product.dn.o, record.product.dn.cn);
    if record.status != "conformant" || record.product.product_type != "generatorProduct" {
        return Err((1, format!("{name} is not a conformant generatorProduct")));
    }
    let level = record.level();
    if level < MIN_EXTERNAL_ASSURANCE {
        return Err((
            2,
            format!("{name} has assurance level {level}, below {MIN_EXTERNAL_ASSURANCE}"),
        ));
    }
    if let Some(min) = record
        .product
        .min_version
        .as_deref()
        .filter(|m| !m.trim().is_empty())
    {
        let min_v = parse_version(min)
            .ok_or((3, format!("{name}: minVersion {min:?} is not comparable")))?;
        let got = info.claim_generator_version.as_deref().ok_or((
            3,
            format!("{name}: claim generator has no version, minimum is {min}"),
        ))?;
        let got_v = parse_version(got).ok_or((
            3,
            format!("{name}: claim generator version {got:?} is not comparable"),
        ))?;
        if compare_versions(&got_v, &min_v) == std::cmp::Ordering::Less {
            return Err((
                3,
                format!("{name}: version {got} is below the minimum {min}"),
            ));
        }
    }
    let mime = info.mime.trim().to_ascii_lowercase();
    let mime_ok = record
        .containers
        .generate
        .values()
        .any(|list| list.iter().any(|m| m.eq_ignore_ascii_case(&mime)));
    if !mime_ok {
        return Err((
            3,
            format!("{name}: MIME type {mime:?} is not in its generate containers"),
        ));
    }
    let created = info.actions.iter().any(|a| {
        a.action == "c2pa.created" && a.digital_source_type.as_deref() == Some(DIGITAL_CAPTURE)
    });
    if !created {
        return Err((
            4,
            "manifest has no c2pa.created with digitalSourceType digitalCapture".into(),
        ));
    }
    for signal in record.disallowed() {
        let hit = info.actions.iter().any(|a| {
            a.action == signal
                || a.digital_source_type.as_deref().is_some_and(|d| {
                    let seg = last_segment(d);
                    seg == signal || iptc_equivalent(signal) == Some(seg)
                })
        }) || info.assertion_labels.iter().any(|l| l == signal);
        if hit {
            return Err((
                4,
                format!("{name}: signal {signal} is disallowed for this product"),
            ));
        }
    }
    Ok(ExternalProduct {
        organization: record.product.dn.o.clone(),
        common_name: record.product.dn.cn.clone(),
        min_version: record
            .product
            .min_version
            .clone()
            .filter(|m| !m.trim().is_empty()),
        assurance_level: level,
    })
}

/// Dotted numeric (`1.10.2`, `v3.5`) or a single integer build number.
fn parse_version(raw: &str) -> Option<Vec<u64>> {
    let s = raw.trim();
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    if s.is_empty() {
        return None;
    }
    s.split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                None
            } else {
                part.parse::<u64>().ok()
            }
        })
        .collect()
}

fn compare_versions(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

/// Verdict for an external capture against the vendored official lists.
pub fn external_capture_verdict(manifest_info: &ExternalManifestInfo) -> ExternalVerdict {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    match TrustStore::vendored() {
        Ok(store) => store.verdict_at(manifest_info, now),
        Err(e) => ExternalVerdict::Unknown {
            reason: format!("vendored C2PA lists unavailable: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::{
        BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose,
    };
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use time::OffsetDateTime;

    const NOW: i64 = 1_800_000_000; // fixed "now" inside every test cert window

    struct Pki {
        root: Certificate,
        root_key: KeyPair,
    }

    fn params(cn: &str, o: Option<&str>, ca: bool) -> CertificateParams {
        let mut p = CertificateParams::new(vec![]).unwrap();
        p.distinguished_name.push(DnType::CommonName, cn);
        if let Some(o) = o {
            p.distinguished_name.push(DnType::OrganizationName, o);
        }
        if ca {
            p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            p.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        } else {
            p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        }
        p.not_before = OffsetDateTime::from_unix_timestamp(NOW - 86_400 * 30).unwrap();
        p.not_after = OffsetDateTime::from_unix_timestamp(NOW + 86_400 * 30).unwrap();
        p
    }

    fn new_root(cn: &str) -> Pki {
        let root_key = KeyPair::generate().unwrap();
        let root = params(cn, Some("Test Root"), true)
            .self_signed(&root_key)
            .unwrap();
        Pki { root, root_key }
    }

    fn leaf(pki: &Pki, o: &str, cn: &str) -> Vec<u8> {
        leaf_with(pki, o, cn, |_| {})
    }

    fn leaf_with(
        pki: &Pki,
        o: &str,
        cn: &str,
        tweak: impl FnOnce(&mut CertificateParams),
    ) -> Vec<u8> {
        let key = KeyPair::generate().unwrap();
        let mut p = params(cn, Some(o), false);
        tweak(&mut p);
        p.signed_by(&key, &pki.root, &pki.root_key)
            .unwrap()
            .der()
            .to_vec()
    }

    fn record(o: &str, cn: &str, level: u8) -> Value {
        json!({
            "status": "conformant",
            "product": {
                "productType": "generatorProduct",
                "DN": {"CN": cn, "O": o, "OU": "", "C": "US"},
                "minVersion": "",
                "assurance": {"maxAssuranceLevel": level, "attestationMethods": []},
                "disallowedSignals": {"inception": [], "transformation": []}
            },
            "containers": {"generate": {"image": ["image/jpeg"]}}
        })
    }

    fn store(pki: &Pki, records: Vec<Value>) -> TrustStore {
        TrustStore::new(
            vec![pki.root.der().to_vec()],
            &serde_json::to_vec(&records).unwrap(),
        )
        .unwrap()
    }

    fn info(chain: Vec<Vec<u8>>) -> ExternalManifestInfo {
        ExternalManifestInfo {
            cert_chain_der: chain,
            signing_time_unix: None,
            claim_generator_name: "Test Camera".into(),
            claim_generator_version: Some("5".into()),
            mime: "image/jpeg".into(),
            actions: vec![ExternalAction {
                action: "c2pa.created".into(),
                digital_source_type: Some(DIGITAL_CAPTURE.into()),
            }],
            assertion_labels: vec![],
        }
    }

    fn is_hm(v: &ExternalVerdict) -> bool {
        matches!(v, ExternalVerdict::Hm { .. })
    }

    fn reason(v: ExternalVerdict) -> String {
        match v {
            ExternalVerdict::Unknown { reason } => reason,
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    const O: &str = "Acme Imaging";
    const CN: &str = "Acme Camera";

    #[test]
    fn baseline_passes() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let v = s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW);
        match v {
            ExternalVerdict::Hm { product } => {
                assert_eq!(product.organization, O);
                assert_eq!(product.common_name, CN);
                assert_eq!(product.assurance_level, 2);
            }
            other => panic!("{other:?}"),
        }
    }

    // Condition 1: chain to the anchors.

    #[test]
    fn chain_to_untrusted_root_is_unknown() {
        let trusted = new_root("Trusted");
        let rogue = new_root("Rogue");
        let s = store(&trusted, vec![record(O, CN, 2)]);
        let v = s.verdict_at(&info(vec![leaf(&rogue, O, CN)]), NOW);
        assert!(reason(v).contains("Trust List"));
    }

    #[test]
    fn empty_and_garbage_chains_are_unknown() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        assert!(!is_hm(&s.verdict_at(&info(vec![]), NOW)));
        assert!(!is_hm(&s.verdict_at(&info(vec![vec![1, 2, 3]]), NOW)));
    }

    #[test]
    fn chain_through_an_intermediate_passes() {
        let pki = new_root("Test Root");
        let ica_key = KeyPair::generate().unwrap();
        let ica = params("Test ICA", Some("Test Root"), true)
            .signed_by(&ica_key, &pki.root, &pki.root_key)
            .unwrap();
        let leaf_key = KeyPair::generate().unwrap();
        let leaf = params(CN, Some(O), false)
            .signed_by(&leaf_key, &ica, &ica_key)
            .unwrap();
        let s = store(&pki, vec![record(O, CN, 2)]);
        let chain = vec![leaf.der().to_vec(), ica.der().to_vec()];
        assert!(is_hm(&s.verdict_at(&info(chain.clone()), NOW)));
        // Without the intermediate the leaf cannot be linked.
        assert!(!is_hm(&s.verdict_at(&info(vec![chain[0].clone()]), NOW)));
    }

    #[test]
    fn forged_intermediate_signature_is_unknown() {
        let pki = new_root("Test Root");
        let other = new_root("Test Root"); // same DN, different key
        let ica_key = KeyPair::generate().unwrap();
        let ica = params("Test ICA", Some("Test Root"), true)
            .signed_by(&ica_key, &other.root, &other.root_key)
            .unwrap();
        let leaf_key = KeyPair::generate().unwrap();
        let leaf = params(CN, Some(O), false)
            .signed_by(&leaf_key, &ica, &ica_key)
            .unwrap();
        let s = store(&pki, vec![record(O, CN, 2)]);
        let chain = vec![leaf.der().to_vec(), ica.der().to_vec()];
        assert!(!is_hm(&s.verdict_at(&info(chain), NOW)));
    }

    #[test]
    fn expired_cert_uses_signing_time_when_present() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let chain = vec![leaf(&pki, O, CN)];
        // Current time after expiry, no trusted timestamp.
        let late = NOW + 86_400 * 365;
        assert!(reason(s.verdict_at(&info(chain.clone()), late)).contains("not valid"));
        // Trusted timestamp inside the validity window.
        let mut i = info(chain);
        i.signing_time_unix = Some(NOW);
        assert!(is_hm(&s.verdict_at(&i, late)));
        // Timestamp before the certificate existed.
        i.signing_time_unix = Some(NOW - 86_400 * 365);
        assert!(!is_hm(&s.verdict_at(&i, late)));
    }

    #[test]
    fn leaf_that_is_a_ca_is_unknown() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let key = KeyPair::generate().unwrap();
        let ca_leaf = params(CN, Some(O), true)
            .signed_by(&key, &pki.root, &pki.root_key)
            .unwrap();
        assert!(!is_hm(
            &s.verdict_at(&info(vec![ca_leaf.der().to_vec()]), NOW)
        ));
    }

    // Condition 2: DN and record kind.

    #[test]
    fn dn_must_match_o_and_cn() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        assert!(
            reason(s.verdict_at(&info(vec![leaf(&pki, "Other Corp", CN)]), NOW))
                .contains("not in the C2PA")
        );
        assert!(
            reason(s.verdict_at(&info(vec![leaf(&pki, O, "Other Camera")]), NOW))
                .contains("not in the C2PA")
        );
    }

    #[test]
    fn validator_or_nonconformant_records_do_not_count() {
        let pki = new_root("Test Root");
        let mut validator = record(O, CN, 2);
        validator["product"]["productType"] = json!("validatorProduct");
        let mut revoked = record(O, CN, 2);
        revoked["status"] = json!("withdrawn");
        let s = store(&pki, vec![validator, revoked]);
        assert!(
            reason(s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW)).contains("generatorProduct")
        );
    }

    // Condition 3: assurance.

    #[test]
    fn assurance_below_two_is_unknown() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 1)]);
        assert!(
            reason(s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW)).contains("assurance level 1")
        );
        // A record without an assurance block counts as level 0.
        let mut none = record(O, CN, 2);
        none["product"].as_object_mut().unwrap().remove("assurance");
        let s = store(&pki, vec![none]);
        assert!(!is_hm(&s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW)));
    }

    #[test]
    fn several_records_for_one_dn_any_passing_record_accepts() {
        let pki = new_root("Test Root");
        let low = record(O, CN, 1);
        let mut high = record(O, CN, 2);
        high["product"]["minVersion"] = json!("3");
        let s = store(&pki, vec![low, high]);
        assert!(is_hm(&s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW)));
    }

    // Condition 4: version and MIME.

    fn with_min(min: &str) -> Value {
        let mut r = record(O, CN, 2);
        r["product"]["minVersion"] = json!(min);
        r
    }

    fn run_version(min: &str, got: Option<&str>) -> ExternalVerdict {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![with_min(min)]);
        let mut i = info(vec![leaf(&pki, O, CN)]);
        i.claim_generator_version = got.map(str::to_owned);
        s.verdict_at(&i, NOW)
    }

    #[test]
    fn version_comparison_rules() {
        assert!(is_hm(&run_version("2.2", Some("2.2"))));
        assert!(
            is_hm(&run_version("2.2", Some("2.10"))),
            "numeric, not lexical"
        );
        assert!(is_hm(&run_version("2.2", Some("3"))));
        assert!(is_hm(&run_version("v3.5", Some("3.5.0"))));
        assert!(!is_hm(&run_version("2.10", Some("2.9"))));
        assert!(!is_hm(&run_version("2.2", Some("2.1.9"))));
        // Integer build numbers, like Pixel.
        assert!(is_hm(&run_version("69307851", Some("69307851"))));
        assert!(is_hm(&run_version("68566598", Some("69307851"))));
        assert!(!is_hm(&run_version("69307851", Some("69134609"))));
        // Unparseable: reject.
        assert!(reason(run_version("2.2", Some("abc"))).contains("not comparable"));
        assert!(reason(run_version("2.2", Some("2.2-beta"))).contains("not comparable"));
        assert!(reason(run_version("2.2", None)).contains("no version"));
        assert!(reason(run_version("weird", Some("2.2"))).contains("not comparable"));
        // No minimum: any (or no) version is fine.
        assert!(is_hm(&run_version("", None)));
        assert!(is_hm(&run_version("", Some("junk"))));
    }

    #[test]
    fn mime_must_be_in_generate_containers() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let mut i = info(vec![leaf(&pki, O, CN)]);
        i.mime = "video/mp4".into();
        assert!(reason(s.verdict_at(&i, NOW)).contains("MIME"));
        i.mime = "IMAGE/JPEG".into();
        assert!(is_hm(&s.verdict_at(&i, NOW)));
        // Records without containers accept nothing.
        let mut bare = record(O, CN, 2);
        bare.as_object_mut().unwrap().remove("containers");
        let s = store(&pki, vec![bare]);
        i.mime = "image/jpeg".into();
        assert!(!is_hm(&s.verdict_at(&i, NOW)));
    }

    // Condition 5: digital capture and disallowed signals.

    #[test]
    fn needs_created_with_digital_capture() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let chain = vec![leaf(&pki, O, CN)];
        let mut i = info(chain);
        i.actions.clear();
        assert!(reason(s.verdict_at(&i, NOW)).contains("c2pa.created"));
        i.actions = vec![ExternalAction {
            action: "c2pa.created".into(),
            digital_source_type: Some(crate::TRAINED_ALGORITHMIC.into()),
        }];
        assert!(!is_hm(&s.verdict_at(&i, NOW)));
        i.actions = vec![ExternalAction {
            action: "c2pa.created".into(),
            digital_source_type: None,
        }];
        assert!(!is_hm(&s.verdict_at(&i, NOW)));
        // digitalCapture on a different action does not count.
        i.actions = vec![ExternalAction {
            action: "c2pa.edited".into(),
            digital_source_type: Some(DIGITAL_CAPTURE.into()),
        }];
        assert!(!is_hm(&s.verdict_at(&i, NOW)));
    }

    #[test]
    fn disallowed_signals_reject() {
        let pki = new_root("Test Root");
        let chain = vec![leaf(&pki, O, CN)];
        let mut base = info(chain);

        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["transformation"] = json!(["c2pa.resized"]);
        let s = store(&pki, vec![r]);
        assert!(is_hm(&s.verdict_at(&base, NOW)));
        base.actions.push(ExternalAction {
            action: "c2pa.resized".into(),
            digital_source_type: None,
        });
        assert!(reason(s.verdict_at(&base, NOW)).contains("c2pa.resized"));

        // A disallowed assertion label.
        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["transformation"] = json!(["stds.exif"]);
        let s = store(&pki, vec![r]);
        let mut i = info(base.cert_chain_der.clone());
        assert!(is_hm(&s.verdict_at(&i, NOW)));
        i.assertion_labels.push("stds.exif".into());
        assert!(!is_hm(&s.verdict_at(&i, NOW)));

        // A product barred from claiming captured media can never be HM.
        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["inception"] = json!(["capturedMedia"]);
        let s = store(&pki, vec![r]);
        assert!(
            reason(s.verdict_at(&info(base.cert_chain_der.clone()), NOW)).contains("capturedMedia")
        );
        assert!(s.eligible_products().is_empty());
    }

    // Production data.

    #[test]
    fn vendored_lock_hashes_match() {
        let lock = include_str!("../../../trust/c2pa/SOURCE.lock");
        let files: [(&str, &[u8]); 3] = [
            ("C2PA-TRUST-LIST.pem", TRUST_LIST_PEM),
            (
                "C2PA-TSA-TRUST-LIST.pem",
                include_bytes!("../../../trust/c2pa/C2PA-TSA-TRUST-LIST.pem"),
            ),
            ("conforming-products-list.json", PRODUCTS_JSON),
        ];
        assert!(lock.contains("repo = \"c2pa-org/conformance-public\""));
        let commit = lock
            .lines()
            .find_map(|l| l.strip_prefix("commit = \""))
            .and_then(|l| l.strip_suffix('"'))
            .expect("commit in SOURCE.lock");
        assert_eq!(commit.len(), 40);
        assert!(commit.bytes().all(|b| b.is_ascii_hexdigit()));
        for (name, bytes) in files {
            let mut hex = String::new();
            for b in Sha256::digest(bytes) {
                hex.push_str(&format!("{b:02x}"));
            }
            let block = lock
                .split("[[file]]")
                .find(|b| b.contains(&format!("name = \"{name}\"")))
                .unwrap_or_else(|| panic!("{name} missing from SOURCE.lock"));
            assert!(
                block.contains(&format!("sha256 = \"{hex}\"")),
                "{name} does not match SOURCE.lock; rerun scripts/update-c2pa-trust.py"
            );
        }
    }

    #[test]
    fn vendored_store_loads_every_anchor() {
        let store = TrustStore::vendored().expect("vendored store");
        assert_eq!(store.anchors.len(), 30);
        for der in &store.anchors {
            let (rest, cert) = X509Certificate::from_der(der).expect("anchor parses");
            assert!(rest.is_empty());
            assert!(cert.is_ca());
        }
    }

    #[test]
    fn eligible_set_at_pinned_commit() {
        let store = TrustStore::vendored().unwrap();
        let got: Vec<String> = store
            .eligible_products()
            .into_iter()
            .map(|(o, c)| format!("{o} / {c}"))
            .collect();
        let want = [
            "EZDRM, INC / DynamicSigner",
            "Evergreen Labs / GreenCheckmark - Android",
            "Google LLC / Pixel Camera",
            "Nuevo.Studio LLC / VWFNDR MBL Android",
            "Qualcomm Technologies, Inc. / Snapdragon 8 Elite Gen 5",
            "Rheely Inc. / Rheely for Android",
            "UX Vision LLC / Stroma",
            "Whole Earth Labs LLC / RealReel Android",
        ];
        assert_eq!(got, want);
        // Brand cameras outside the conformance program are never eligible.
        for brand in ["Leica", "Sony", "Nikon", "Canon", "Samsung", "Apple"] {
            assert!(
                !got.iter()
                    .any(|p| p.to_lowercase().contains(&brand.to_lowercase())),
                "{brand} must stay Unknown"
            );
        }
    }

    #[test]
    fn production_path_rejects_an_untrusted_signer() {
        let pki = new_root("Rogue Root");
        let v = external_capture_verdict(&info(vec![leaf(&pki, "Google LLC", "Pixel Camera")]));
        assert!(!is_hm(&v));
    }

    #[test]
    fn vendored_roots_self_verify() {
        // At the pinned commit only the vivo root (non-ring algorithm) is not verifiable.
        // Roots whose algorithm ring cannot verify would make their chains
        // fail closed. Intermediates in the list are matched by DER equality
        // or by their issuer being another anchor, so only roots are checked.
        let store = TrustStore::vendored().unwrap();
        let mut roots = 0;
        let mut unverifiable = Vec::new();
        for der in &store.anchors {
            let (_, cert) = X509Certificate::from_der(der).unwrap();
            if cert.subject().as_raw() == cert.issuer().as_raw() {
                roots += 1;
                if cert.verify_signature(None).is_err() {
                    unverifiable.push(cert.subject().to_string());
                }
            }
        }
        eprintln!("roots: {roots}, not verifiable: {unverifiable:?}");
        assert!(roots > 0);
        assert!(unverifiable.len() * 4 < roots, "{unverifiable:?}");
    }
}
