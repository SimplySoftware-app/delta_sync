//! Deterministic acceptance of EXTERNAL cameras via the official C2PA lists.
//!
//! An external capture (a manifest NOT signed by our own device authority) is
//! HM only if every condition below holds. Otherwise the verdict is
//! `Unknown`. Nothing is guessed.
//!
//! 1. The signer certificate chains to the pinned C2PA Trust List anchors.
//!    The leaf is a non-CA certificate with `digitalSignature` key usage and
//!    the `c2pa-kp-claimSigning` or `id-kp-documentSigning` EKU; every
//!    intermediate is a CA and respects its `pathLen`.
//! 2. The signer certificate subject has exactly one `O` and one `CN`, and
//!    `O`, `CN`, `C` (and `OU` when the record has one) match a conformance
//!    record with `productType == generatorProduct` and
//!    `status == conformant`.
//! 3. That record's `maxAssuranceLevel` is at least [`MIN_EXTERNAL_ASSURANCE`].
//! 4. The claim generator version is at least the record's `minVersion` (when
//!    set) and the asset MIME type is in the record's `containers.generate`.
//! 5. The manifest has exactly one inception action (`c2pa.created` or
//!    `c2pa.opened`), it is `c2pa.created` with `digitalSourceType`
//!    digitalCapture, no action carries any other `digitalSourceType`, and no
//!    action, source type or assertion is named in `disallowedSignals`. A
//!    signal name this code does not recognise fails closed.
//! 6. (Label integration, later) Subsequent edits must be permitted actions
//!    or our own service seal. Not decided here.
//!
//! The variant name [`ExternalVerdict::Hm`] is historical. This function only
//! says that the *capture origin* is eligible to be called human-made. Edits
//! after capture (rule 6) are judged by the label integration, which may still
//! downgrade the final label.
//!
//! The lists are vendored under `trust/c2pa/` and embedded at compile time, so
//! the verdict of a given binary is fixed by its build. `SOURCE.lock` pins the
//! upstream commit and the sha256 of each file; a test fails if they drift.
//!
//! Not checked here: certificate revocation (CRL/OCSP). Validity is always
//! evaluated at the current time: no caller-supplied signing time is trusted.
//! RFC 3161 timestamp tokens (which would allow validating at the original
//! signing time against `C2PA-TSA-TRUST-LIST.pem`) are not verified yet, so a
//! capture whose certificate has since expired is `Unknown`. The TSA list is
//! vendored and hash-pinned for that future step.
//!
//! Signature algorithms are those `x509-parser` verifies through `ring`
//! (RSA PKCS#1, ECDSA P-256/P-384 with SHA-256/384, Ed25519). A chain that
//! needs anything else (for instance RSA-PSS) fails closed as `Unknown`.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use x509_parser::certificate::X509Certificate;
use x509_parser::der_parser::oid;
use x509_parser::der_parser::oid::Oid;
use x509_parser::oid_registry::{OID_PKCS1_SHA1WITHRSA, OID_SHA1_WITH_RSA};
use x509_parser::pem::Pem;
use x509_parser::prelude::FromDer;
use x509_parser::time::ASN1Time;
use x509_parser::x509::X509Name;

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
    pub claim_generator_name: String,
    pub claim_generator_version: Option<String>,
    pub mime: String,
    pub actions: Vec<ExternalAction>,
    /// Labels of the other assertions in the manifest (a trailing `__N`
    /// version suffix is ignored when comparing).
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
    #[serde(rename = "OU", default)]
    ou: String,
    #[serde(rename = "C", default)]
    c: String,
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
                    && r.disallowed().all(signal_recognised)
                    && !r.disallowed().any(signal_matches_capture)
            })
            .map(|r| (r.product.dn.o.clone(), r.product.dn.cn.clone()))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    pub(crate) fn verdict_at(&self, info: &ExternalManifestInfo, at_unix: i64) -> ExternalVerdict {
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
        // 1. Chain to the pinned anchors, evaluated at the current time.
        let subject = self.validate_chain(&info.cert_chain_der, at_unix)?;
        // 5 (record independent): capture origin.
        check_inception(&info.actions)?;

        // 2-5, evaluated per record: the same DN can appear in several
        // versions. Any record that passes everything accepts the capture;
        // otherwise report the failure that got furthest.
        let candidates: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| subject.matches(&r.product.dn))
            .collect();
        if candidates.is_empty() {
            return Err(format!(
                "signer {} / {} is not in the C2PA conforming products list",
                subject.o, subject.cn
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

    /// Returns the leaf subject when the chain is trusted at `at`.
    fn validate_chain(&self, chain: &[Vec<u8>], at: i64) -> Result<Subject, String> {
        if chain.is_empty() {
            return Err("manifest has no signer certificate".into());
        }
        let at_time =
            ASN1Time::from_timestamp(at).map_err(|_| "invalid verification time".to_string())?;
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
        check_leaf(leaf)?;
        let trusted = (|| {
            for (i, cert) in certs.iter().enumerate() {
                if !cert.validity().is_valid_at(at_time) {
                    return Err(format!(
                        "certificate {i} of the chain is not valid at the verification time"
                    ));
                }
                if i > 0 {
                    // CA certificates below this one: certs 1..i-1.
                    check_ca(cert, i - 1)
                        .map_err(|e| format!("certificate {i} of the chain {e}"))?;
                    match cert.key_usage() {
                        Ok(Some(ku)) if ku.value.key_cert_sign() => {}
                        _ => {
                            return Err(format!(
                                "certificate {i} of the chain lacks the keyCertSign key usage"
                            ))
                        }
                    }
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
                        && check_ca(a, i).is_ok()
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

        parse_subject(leaf.subject())
    }
}

/// Leaf subject attributes used to find the conformance record.
#[derive(Debug, PartialEq, Eq)]
struct Subject {
    o: String,
    cn: String,
    c: Option<String>,
    ou: Option<String>,
}

impl Subject {
    /// `O`, `CN` and `C` must equal the record's; `OU` too when the record
    /// has one. An absent `C` only matches an empty record `C`.
    fn matches(&self, dn: &Dn) -> bool {
        self.o == dn.o
            && self.cn == dn.cn
            && self.c.as_deref().unwrap_or("") == dn.c
            && (dn.ou.is_empty() || self.ou.as_deref() == Some(dn.ou.as_str()))
    }
}

fn single_attr<'a>(
    mut iter: impl Iterator<Item = &'a x509_parser::x509::AttributeTypeAndValue<'a>>,
    name: &str,
    required: bool,
) -> Result<Option<String>, String> {
    let first = iter.next();
    if iter.next().is_some() {
        return Err(format!(
            "signer certificate subject has more than one {name}"
        ));
    }
    match first {
        None if required => Err(format!("signer certificate subject lacks {name}")),
        None => Ok(None),
        Some(a) => a
            .as_str()
            .map(|v| Some(v.to_owned()))
            .map_err(|_| format!("signer certificate subject {name} is not a string")),
    }
}

/// Exactly one `O` and one `CN`; at most one `C` and `OU` (more is ambiguous
/// and fails closed).
fn parse_subject(name: &X509Name) -> Result<Subject, String> {
    Ok(Subject {
        o: single_attr(name.iter_organization(), "O", true)?.unwrap_or_default(),
        cn: single_attr(name.iter_common_name(), "CN", true)?.unwrap_or_default(),
        c: single_attr(name.iter_country(), "C", false)?,
        ou: single_attr(name.iter_organizational_unit(), "OU", false)?,
    })
}

/// c2pa-kp-claimSigning.
const EKU_C2PA_CLAIM_SIGNING: Oid<'static> = oid!(1.3.6 .1 .4 .1 .62558 .2 .1);
/// id-kp-documentSigning.
const EKU_DOCUMENT_SIGNING: Oid<'static> = oid!(1.3.6 .1 .5 .5 .7 .3 .36);

/// The signer certificate: not a CA, `digitalSignature`, claim/document
/// signing EKU.
fn check_leaf(leaf: &X509Certificate) -> Result<(), String> {
    if leaf.is_ca() {
        return Err("signer certificate is a CA certificate".into());
    }
    match leaf.key_usage() {
        Ok(Some(ku)) if ku.value.digital_signature() => {}
        _ => return Err("signer certificate lacks the digitalSignature key usage".into()),
    }
    match leaf.extended_key_usage() {
        Ok(Some(eku))
            if !eku.value.any
                && eku
                    .value
                    .other
                    .iter()
                    .any(|o| *o == EKU_C2PA_CLAIM_SIGNING || *o == EKU_DOCUMENT_SIGNING) => {}
        _ => {
            return Err(
                "signer certificate lacks the c2pa claimSigning or documentSigning EKU".into(),
            )
        }
    }
    Ok(())
}

/// A CA certificate in the chain: `basicConstraints` CA=true, and `pathLen`
/// (when set) allows `below` CA certificates between it and the leaf.
fn check_ca(cert: &X509Certificate, below: usize) -> Result<(), String> {
    match cert.basic_constraints() {
        Ok(Some(bc)) if bc.value.ca => {
            if let Some(max) = bc.value.path_len_constraint {
                if below as u64 > u64::from(max) {
                    return Err(format!("exceeds its pathLen constraint of {max}"));
                }
            }
            Ok(())
        }
        _ => Err("is not a CA".into()),
    }
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
        "capturedMediaStitched" => Some("compositeCapture"),
        "nonGenAIDigitalCreation" => Some("digitalCreation"),
        _ => None,
    }
}

/// A `disallowedSignals` name this code knows how to evaluate, from an
/// explicit allowlist: the inception names and the short IPTC source types
/// they map to, or a namespaced action / assertion label
/// `^(c2pa|stds|cawg)\.[a-z0-9_]+(\.[a-z0-9_]+)*$`. Everything else fails
/// closed: full IPTC URLs, wildcards such as `c2pa.*`, unknown namespaces.
fn signal_recognised(signal: &str) -> bool {
    matches!(
        signal,
        "capturedMedia"
            | "capturedMediaStitched"
            | "nonGenAIDigitalCreation"
            | "digitalCapture"
            | "compositeCapture"
            | "digitalCreation"
    ) || is_namespaced_label(signal)
}

fn is_namespaced_label(s: &str) -> bool {
    let Some((ns, rest)) = s.split_once('.') else {
        return false;
    };
    matches!(ns, "c2pa" | "stds" | "cawg")
        && rest.split('.').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

/// Label or action name without a trailing `__N` and then `.vN` version
/// suffix, so `c2pa.actions.v2` and `c2pa.actions` compare equal.
fn strip_label_version(label: &str) -> &str {
    let label = match label.rsplit_once("__") {
        Some((base, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => label,
    };
    match label.rsplit_once(".v") {
        Some((base, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => label,
    }
}

/// Rule 5, independent of the record: one inception action, a digital
/// capture, and no other `digitalSourceType` anywhere in the manifest.
fn check_inception(actions: &[ExternalAction]) -> Result<(), String> {
    let inceptions = actions
        .iter()
        .filter(|a| a.action == "c2pa.created" || a.action == "c2pa.opened")
        .count();
    if inceptions > 1 {
        return Err("manifest has more than one c2pa.created/c2pa.opened action".into());
    }
    if let Some(other) = actions
        .iter()
        .filter_map(|a| a.digital_source_type.as_deref())
        .find(|d| *d != DIGITAL_CAPTURE)
    {
        return Err(format!(
            "an action carries digitalSourceType {other:?}, not digitalCapture"
        ));
    }
    Ok(())
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
        if !signal_recognised(signal) {
            return Err((
                4,
                format!("{name}: disallowed signal {signal:?} is not recognised"),
            ));
        }
        let sig = strip_label_version(signal);
        let hit = info.actions.iter().any(|a| {
            strip_label_version(&a.action) == sig
                || a.digital_source_type.as_deref().is_some_and(|d| {
                    let seg = last_segment(d);
                    seg == sig || iptc_equivalent(sig) == Some(seg)
                })
        }) || info
            .assertion_labels
            .iter()
            .any(|l| strip_label_version(l) == sig);
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
///
/// The caller MUST pass data taken from a manifest that c2pa-rs has already
/// validated, including the claim signature and the hard bindings (the
/// asset hash), and `mime` MUST come from the actual asset bytes, not from a
/// file name or a header. This function trusts those inputs: it checks the
/// signer chain, the conformance record and the capture claims, never the
/// manifest signature or the asset itself.
///
/// `Hm` means only that the capture origin is eligible. Edits after capture
/// are judged by the label integration (rule 6).
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
        BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
        KeyPair, KeyUsagePurpose,
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
        p.distinguished_name.push(DnType::CountryName, "US");
        if let Some(o) = o {
            p.distinguished_name.push(DnType::OrganizationName, o);
        }
        if ca {
            p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            p.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        } else {
            p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
            p.extended_key_usages = vec![ExtendedKeyUsagePurpose::Other(vec![
                1, 3, 6, 1, 4, 1, 62558, 2, 1,
            ])];
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
    fn validity_is_checked_at_the_given_time_only() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let chain = vec![leaf(&pki, O, CN)];
        assert!(is_hm(&s.verdict_at(&info(chain.clone()), NOW)));
        // After expiry: Unknown. No caller-supplied time can rescue it.
        let late = NOW + 86_400 * 365;
        assert!(reason(s.verdict_at(&info(chain.clone()), late)).contains("not valid"));
        // Before the certificate existed.
        let early = NOW - 86_400 * 365;
        assert!(!is_hm(&s.verdict_at(&info(chain), early)));
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

    // Leaf and intermediate profile.

    fn eku_leaf(pki: &Pki, eku: Vec<ExtendedKeyUsagePurpose>) -> Vec<u8> {
        leaf_with(pki, O, CN, |p| p.extended_key_usages = eku)
    }

    #[test]
    fn leaf_needs_claim_or_document_signing_eku() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        // rcgen writes a malformed tail when the EKU is empty, so add a
        // harmless custom extension after it to keep the certificate valid.
        let none = leaf_with(&pki, O, CN, |p| {
            p.extended_key_usages = vec![];
            p.custom_extensions = vec![rcgen::CustomExtension::from_oid_content(
                &[1, 2, 3, 4],
                vec![0x05, 0x00],
            )];
        });
        let r = reason(s.verdict_at(&info(vec![none]), NOW));
        assert!(r.contains("EKU"), "{r}");
        let server = eku_leaf(&pki, vec![ExtendedKeyUsagePurpose::ServerAuth]);
        assert!(!is_hm(&s.verdict_at(&info(vec![server]), NOW)));
        let doc = eku_leaf(
            &pki,
            vec![ExtendedKeyUsagePurpose::Other(vec![
                1, 3, 6, 1, 5, 5, 7, 3, 36,
            ])],
        );
        assert!(is_hm(&s.verdict_at(&info(vec![doc]), NOW)));
    }

    #[test]
    fn leaf_with_any_extended_key_usage_is_rejected() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let any = eku_leaf(
            &pki,
            vec![
                ExtendedKeyUsagePurpose::Any,
                ExtendedKeyUsagePurpose::Other(vec![1, 3, 6, 1, 4, 1, 62558, 2, 1]),
            ],
        );
        assert!(!is_hm(&s.verdict_at(&info(vec![any]), NOW)));
    }

    #[test]
    fn intermediate_needs_key_cert_sign() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let k = KeyPair::generate().unwrap();
        let mut p = params("ICA", Some("Test Root"), true);
        p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        let ica = p.signed_by(&k, &pki.root, &pki.root_key).unwrap();
        let kl = KeyPair::generate().unwrap();
        let leaf = params(CN, Some(O), false).signed_by(&kl, &ica, &k).unwrap();
        let chain = vec![leaf.der().to_vec(), ica.der().to_vec()];
        assert!(reason(s.verdict_at(&info(chain), NOW)).contains("keyCertSign"));
    }

    #[test]
    fn leaf_needs_digital_signature_key_usage() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let no_ds = leaf_with(&pki, O, CN, |p| {
            p.key_usages = vec![KeyUsagePurpose::ContentCommitment]
        });
        assert!(reason(s.verdict_at(&info(vec![no_ds]), NOW)).contains("digitalSignature"));
        let none = leaf_with(&pki, O, CN, |p| p.key_usages = vec![]);
        assert!(!is_hm(&s.verdict_at(&info(vec![none]), NOW)));
    }

    /// root -> ICA1 (constraint `c1`) -> ICA2 -> leaf.
    fn two_level_chain(pki: &Pki, c1: IsCa, c2_is_ca: bool) -> Vec<Vec<u8>> {
        let k1 = KeyPair::generate().unwrap();
        let mut p1 = params("ICA 1", Some("Test Root"), true);
        p1.is_ca = c1;
        let ica1 = p1.signed_by(&k1, &pki.root, &pki.root_key).unwrap();
        let k2 = KeyPair::generate().unwrap();
        let mut p2 = params("ICA 2", Some("Test Root"), true);
        if !c2_is_ca {
            p2.is_ca = IsCa::NoCa;
        }
        let ica2 = p2.signed_by(&k2, &ica1, &k1).unwrap();
        let kl = KeyPair::generate().unwrap();
        let leaf = params(CN, Some(O), false)
            .signed_by(&kl, &ica2, &k2)
            .unwrap();
        vec![
            leaf.der().to_vec(),
            ica2.der().to_vec(),
            ica1.der().to_vec(),
        ]
    }

    #[test]
    fn intermediate_must_be_ca_and_respect_path_len() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let ok = two_level_chain(&pki, IsCa::Ca(BasicConstraints::Constrained(1)), true);
        assert!(is_hm(&s.verdict_at(&info(ok), NOW)));
        let too_deep = two_level_chain(&pki, IsCa::Ca(BasicConstraints::Constrained(0)), true);
        assert!(reason(s.verdict_at(&info(too_deep), NOW)).contains("pathLen"));
        let not_ca = two_level_chain(&pki, IsCa::Ca(BasicConstraints::Unconstrained), false);
        assert!(reason(s.verdict_at(&info(not_ca), NOW)).contains("not a CA"));
    }

    #[test]
    fn anchor_path_len_is_enforced() {
        // A pathLen=0 anchor cannot sign an intermediate that signs the leaf.
        let key = KeyPair::generate().unwrap();
        let mut p = params("Limited Root", Some("Test Root"), true);
        p.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        let root = p.self_signed(&key).unwrap();
        let pki = Pki {
            root,
            root_key: key,
        };
        let s = store(&pki, vec![record(O, CN, 2)]);
        let k1 = KeyPair::generate().unwrap();
        let ica = params("ICA", Some("Test Root"), true)
            .signed_by(&k1, &pki.root, &pki.root_key)
            .unwrap();
        let kl = KeyPair::generate().unwrap();
        let leaf = params(CN, Some(O), false)
            .signed_by(&kl, &ica, &k1)
            .unwrap();
        let chain = vec![leaf.der().to_vec(), ica.der().to_vec()];
        assert!(!is_hm(&s.verdict_at(&info(chain), NOW)));
        // Directly under the root it is fine.
        assert!(is_hm(&s.verdict_at(&info(vec![leaf_der(&pki)]), NOW)));
    }

    fn leaf_der(pki: &Pki) -> Vec<u8> {
        leaf(pki, O, CN)
    }

    // Condition 2: DN and record kind.

    /// DER `Name` with the given (oid, value) attributes, one RDN each.
    fn name_der(attrs: &[(&[u8], &str)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (oid, val) in attrs {
            let mut atv = vec![0x06, oid.len() as u8];
            atv.extend_from_slice(oid);
            atv.extend([0x0c, val.len() as u8]);
            atv.extend_from_slice(val.as_bytes());
            let mut seq = vec![0x30, atv.len() as u8];
            seq.extend(atv);
            body.extend([0x31, seq.len() as u8]);
            body.extend(seq);
        }
        let mut out = vec![0x30, body.len() as u8];
        out.extend(body);
        out
    }

    fn subject_of(attrs: &[(&[u8], &str)]) -> Result<Subject, String> {
        let der = name_der(attrs);
        let (_, name) = X509Name::from_der(&der).unwrap();
        parse_subject(&name)
    }

    const OID_O: &[u8] = &[0x55, 0x04, 0x0a];
    const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
    const OID_C: &[u8] = &[0x55, 0x04, 0x06];
    const OID_OU: &[u8] = &[0x55, 0x04, 0x0b];

    #[test]
    fn subject_needs_exactly_one_o_and_one_cn() {
        let ok = subject_of(&[(OID_O, "A"), (OID_CN, "B"), (OID_C, "US")]).unwrap();
        assert_eq!((ok.o.as_str(), ok.cn.as_str()), ("A", "B"));
        assert!(subject_of(&[(OID_O, "A"), (OID_O, "A2"), (OID_CN, "B")])
            .unwrap_err()
            .contains("more than one O"));
        assert!(subject_of(&[(OID_O, "A"), (OID_CN, "B"), (OID_CN, "B2")])
            .unwrap_err()
            .contains("more than one CN"));
        assert!(subject_of(&[(OID_CN, "B")])
            .unwrap_err()
            .contains("lacks O"));
        assert!(subject_of(&[(OID_O, "A")])
            .unwrap_err()
            .contains("lacks CN"));
        assert!(subject_of(&[(OID_O, "A"), (OID_CN, "B"), (OID_C, "US"), (OID_C, "ES")]).is_err());
        assert!(subject_of(&[(OID_O, "A"), (OID_CN, "B"), (OID_OU, "x"), (OID_OU, "y")]).is_err());
    }

    #[test]
    fn country_and_organizational_unit_must_match_the_record() {
        let pki = new_root("Test Root");
        let mut with_ou = record(O, CN, 2);
        with_ou["product"]["DN"]["OU"] = json!("Hardware");
        let s = store(&pki, vec![with_ou]);
        // Record has an OU: the cert must carry the same one.
        let no_ou = leaf(&pki, O, CN);
        assert!(reason(s.verdict_at(&info(vec![no_ou]), NOW)).contains("not in the C2PA"));
        let wrong = leaf_with(&pki, O, CN, |p| {
            p.distinguished_name
                .push(DnType::OrganizationalUnitName, "Software")
        });
        assert!(!is_hm(&s.verdict_at(&info(vec![wrong]), NOW)));
        let right = leaf_with(&pki, O, CN, |p| {
            p.distinguished_name
                .push(DnType::OrganizationalUnitName, "Hardware")
        });
        assert!(is_hm(&s.verdict_at(&info(vec![right]), NOW)));
        // Country mismatch (record says US).
        let s = store(&pki, vec![record(O, CN, 2)]);
        let es = leaf_with(&pki, O, CN, |p| {
            p.distinguished_name.push(DnType::CountryName, "ES")
        });
        assert!(reason(s.verdict_at(&info(vec![es]), NOW)).contains("not in the C2PA"));
    }

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

    #[test]
    fn any_other_digital_source_type_is_rejected() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let base = info(vec![leaf(&pki, O, CN)]);
        assert!(is_hm(&s.verdict_at(&base, NOW)));
        let base_url = "http://cv.iptc.org/newscodes/digitalsourcetype/";
        for t in [
            "trainedAlgorithmicMedia",
            "compositeWithTrainedAlgorithmicMedia",
            "algorithmicMedia",
            "compositeCapture",
            "digitalCreation",
            "composite",
        ] {
            let mut i = base.clone();
            i.actions.push(ExternalAction {
                action: "c2pa.edited".into(),
                digital_source_type: Some(format!("{base_url}{t}")),
            });
            assert!(
                reason(s.verdict_at(&i, NOW)).contains("not digitalCapture"),
                "{t}"
            );
        }
    }

    #[test]
    fn only_one_inception_action_is_allowed() {
        let pki = new_root("Test Root");
        let s = store(&pki, vec![record(O, CN, 2)]);
        let mut i = info(vec![leaf(&pki, O, CN)]);
        i.actions.push(ExternalAction {
            action: "c2pa.opened".into(),
            digital_source_type: None,
        });
        assert!(reason(s.verdict_at(&i, NOW)).contains("more than one"));
        let mut i = info(vec![leaf(&pki, O, CN)]);
        i.actions.push(i.actions[0].clone());
        assert!(!is_hm(&s.verdict_at(&i, NOW)));
    }

    #[test]
    fn stitched_maps_to_composite_capture_and_labels_ignore_version() {
        assert_eq!(
            iptc_equivalent("capturedMediaStitched"),
            Some("compositeCapture")
        );
        assert_eq!(strip_label_version("stds.exif__2"), "stds.exif");
        assert_eq!(strip_label_version("stds.exif"), "stds.exif");
        assert_eq!(strip_label_version("a__b"), "a__b");
        let pki = new_root("Test Root");
        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["transformation"] = json!(["stds.exif"]);
        let s = store(&pki, vec![r]);
        let mut i = info(vec![leaf(&pki, O, CN)]);
        i.assertion_labels.push("stds.exif__2".into());
        assert!(reason(s.verdict_at(&i, NOW)).contains("stds.exif"));
        // A stitched-only restriction does not make the product ineligible:
        // compositeCapture is already rejected for every external capture.
        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["inception"] = json!(["capturedMediaStitched"]);
        let s = store(&pki, vec![r]);
        assert_eq!(s.eligible_products().len(), 1);
    }

    fn signal_record(signals: Value) -> Value {
        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["transformation"] = signals;
        r
    }

    #[test]
    fn probe_full_iptc_url_signal_fails_closed() {
        let pki = new_root("Test Root");
        let url = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture";
        let s = store(&pki, vec![signal_record(json!([url]))]);
        let v = s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW);
        assert!(reason(v).contains("not recognised"));
        assert!(s.eligible_products().is_empty());
    }

    #[test]
    fn probe_versioned_labels_match_both_ways() {
        let pki = new_root("Test Root");
        for (signal, label) in [
            ("c2pa.actions", "c2pa.actions.v2"),
            ("c2pa.actions.v2", "c2pa.actions"),
            ("stds.exif", "stds.exif__3"),
        ] {
            let s = store(&pki, vec![signal_record(json!([signal]))]);
            let mut i = info(vec![leaf(&pki, O, CN)]);
            i.assertion_labels.push(label.into());
            assert!(
                reason(s.verdict_at(&i, NOW)).contains("disallowed"),
                "{signal} vs {label}"
            );
        }
        assert_eq!(strip_label_version("c2pa.actions.v2"), "c2pa.actions");
        assert_eq!(strip_label_version("c2pa.actions.v2__1"), "c2pa.actions");
        assert_eq!(strip_label_version("c2pa.vivid"), "c2pa.vivid");
    }

    #[test]
    fn probe_wildcards_fail_closed() {
        let pki = new_root("Test Root");
        for sig in ["c2pa.*", "*", "c2pa.", "c2pa..x", "c2pa.A"] {
            let s = store(&pki, vec![signal_record(json!([sig]))]);
            let v = s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW);
            assert!(reason(v).contains("not recognised"), "{sig}");
            assert!(s.eligible_products().is_empty());
        }
    }

    #[test]
    fn probe_unknown_namespaces_fail_closed() {
        let pki = new_root("Test Root");
        for sig in ["foo.bar", "x.c2pa.actions", "capturedmedia"] {
            let s = store(&pki, vec![signal_record(json!([sig]))]);
            let v = s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW);
            assert!(reason(v).contains("not recognised"), "{sig}");
        }
        // Hyphens and capitals are outside the allowlist pattern.
        assert!(is_namespaced_label("c2pa.resized"));
        assert!(!is_namespaced_label("cawg.training-mining"));
        assert!(!is_namespaced_label("stds.schema-org.CreativeWork"));
    }

    #[test]
    fn python_updater_agrees_on_probe_signals() {
        let probes = [
            json!(["c2pa.resized"]),
            json!(["http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"]),
            json!(["c2pa.*"]),
            json!(["c2pa.actions.v2"]),
            json!(["foo.bar"]),
            json!(["capturedMediaStitched"]),
            json!(["capturedMedia"]),
            json!(["digitalCapture"]),
            json!(["compositeCapture", "stds.exif"]),
            json!(["somethingNew"]),
            json!(["cawg.training-mining"]),
            json!([]),
        ];
        let pki = new_root("Test Root");
        let records: Vec<Value> = probes
            .iter()
            .enumerate()
            .map(|(n, sigs)| {
                let mut r = signal_record(sigs.clone());
                r["product"]["DN"]["CN"] = json!(format!("Cam {n:02}"));
                r
            })
            .collect();
        let body = serde_json::to_vec(&records).unwrap();
        let path = std::env::temp_dir().join(format!("c2pa-probe-{}.json", std::process::id()));
        std::fs::write(&path, &body).unwrap();
        let py = python_eligible(Some(path.to_str().unwrap()));
        std::fs::remove_file(&path).ok();
        let rust: Vec<String> = store(&pki, records)
            .eligible_products()
            .into_iter()
            .map(|(o, c)| format!("{o} / {c}"))
            .collect();
        assert_eq!(py, rust);
        assert!(!rust.is_empty() && rust.len() < probes.len());
    }

    #[test]
    fn unrecognised_signal_fails_closed() {
        let pki = new_root("Test Root");
        let mut r = record(O, CN, 2);
        r["product"]["disallowedSignals"]["inception"] = json!(["somethingNew"]);
        let s = store(&pki, vec![r]);
        let v = s.verdict_at(&info(vec![leaf(&pki, O, CN)]), NOW);
        assert!(reason(v).contains("not recognised"));
        assert!(s.eligible_products().is_empty());
    }

    // Production data.

    /// Eligible `O / CN` lines from scripts/update-c2pa-trust.py, optionally
    /// over a different record file.
    fn python_eligible(path: Option<&str>) -> Vec<String> {
        let script = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/update-c2pa-trust.py"
        );
        let mut args = vec!["-I", script, "--list-eligible"];
        args.extend(path);
        let out = std::process::Command::new("python3")
            .args(args)
            .output()
            .expect("python3 is required for the parity test");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn python_updater_agrees_on_the_eligible_set() {
        // scripts/update-c2pa-trust.py decides which products a list update
        // gains or loses. Its filter must equal this crate's.
        let rust: Vec<String> = TrustStore::vendored()
            .unwrap()
            .eligible_products()
            .into_iter()
            .map(|(o, c)| format!("{o} / {c}"))
            .collect();
        assert_eq!(python_eligible(None), rust);
    }

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
