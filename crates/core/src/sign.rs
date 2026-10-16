use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::sync::Arc;

use c2pa::{
    Builder, BuilderIntent, CallbackSigner, ClaimGeneratorInfo, Context, DigitalSourceType, Reader,
    Signer as C2paSigner, SigningAlg,
};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::Signer;
use rcgen::KeyPair;
use serde_json::json;

use crate::device_signer::{c2pa_signer, DeviceSigner};
use crate::edit::Edit;
use crate::error::CoreError;
use crate::keys::{DEVICE_ICA_CN, SERVICE_ICA_CN};
use crate::label::{self, LabelVerdict, SignerRole};
use crate::{GENERATOR_NAME, SEAL_GENERATOR, VERSION};

/// Chunk size of the Merkle tree over the media payload of a BMFF video (MP4,
/// MOV). A reader can then verify a byte range without hashing the whole file.
const MERKLE_CHUNK_KB: u32 = 1024;

/// How a signature is time-stamped (RFC 3161), so it stays valid after the
/// short-lived certificate expires.
#[cfg_attr(feature = "ffi", uniffi::export(with_foreign))]
pub trait TimestampProvider: Send + Sync {
    /// POST `request` (a DER `TimeStampReq`, content type
    /// `application/timestamp-query`) to the time-stamp authority and return
    /// the DER `TimeStampResp`.
    fn timestamp(&self, request: Vec<u8>) -> Result<Vec<u8>, CoreError>;
}

/// Where the time stamp comes from, if anywhere.
#[derive(Clone, Default)]
pub enum Timestamping {
    #[default]
    None,
    /// c2pa fetches it over HTTP. The caller's build must enable a c2pa HTTP
    /// feature (the server does). Without one, signing fails.
    Url(String),
    /// The caller fetches it. Devices use this: the platform makes the request.
    Provider(Arc<dyn TimestampProvider>),
}

/// A c2pa signer that gets its time stamp from a [`TimestampProvider`].
struct ProviderSigner {
    inner: CallbackSigner,
    provider: Arc<dyn TimestampProvider>,
}

impl C2paSigner for ProviderSigner {
    fn sign(&self, data: &[u8]) -> c2pa::Result<Vec<u8>> {
        self.inner.sign(data)
    }

    fn alg(&self) -> SigningAlg {
        self.inner.alg()
    }

    fn certs(&self) -> c2pa::Result<Vec<Vec<u8>>> {
        self.inner.certs()
    }

    fn reserve_size(&self) -> usize {
        // The token carries the TSA's certificate chain.
        self.inner.reserve_size() + 8192
    }

    fn send_timestamp_request(&self, message: &[u8]) -> Option<c2pa::Result<Vec<u8>>> {
        let request = match c2pa::crypto::time_stamp::default_rfc3161_message(message) {
            Ok(request) => request,
            Err(err) => return Some(Err(err.into())),
        };
        Some(
            self.provider
                .timestamp(request)
                .map_err(|err| c2pa::Error::BadParam(format!("time stamp: {err}"))),
        )
    }
}

fn with_timestamping(signer: CallbackSigner, tsa: &Timestamping) -> Box<dyn C2paSigner> {
    match tsa {
        Timestamping::None => Box::new(signer),
        Timestamping::Url(url) => Box::new(signer.set_tsa_url(url)),
        Timestamping::Provider(provider) => Box::new(ProviderSigner {
            inner: signer,
            provider: provider.clone(),
        }),
    }
}

/// What a verifier trusts.
#[derive(Clone, Debug)]
pub struct VerifyOptions {
    /// The root. Both ICAs must chain to it.
    pub ca_pem: String,
    /// Roots of the time-stamp authorities that may vouch for the time of a
    /// signature. Without one, an expired device certificate is not accepted:
    /// a time stamp from an unknown authority proves nothing.
    pub tsa_anchors_pem: Option<String>,
}

impl VerifyOptions {
    pub fn new(ca_pem: impl Into<String>) -> Self {
        Self {
            ca_pem: ca_pem.into(),
            tsa_anchors_pem: None,
        }
    }

    pub fn with_tsa_anchors(mut self, tsa_anchors_pem: impl Into<String>) -> Self {
        self.tsa_anchors_pem = Some(tsa_anchors_pem.into());
        self
    }
}

/// `c2pa-kp-claimSigning`, the EKU on every certificate the authority issues.
const CLAIM_SIGNING_EKU_OID: &str = "1.3.6.1.4.1.62558.2.1";

pub fn sign_capture(
    asset: &[u8],
    mime: &str,
    private_pem: &str,
    cert_pem: &str,
    ca_pem: &str,
) -> Result<Vec<u8>, CoreError> {
    sign(asset, mime, private_pem, cert_pem, ca_pem, None)
}

pub fn sign_edit(
    asset: &[u8],
    mime: &str,
    private_pem: &str,
    cert_pem: &str,
    ca_pem: &str,
    edit: &Edit,
) -> Result<Vec<u8>, CoreError> {
    edit.validate()?;
    sign(asset, mime, private_pem, cert_pem, ca_pem, Some(edit))
}

/// Seal a rendered derivative, carrying the source's manifest chain as its parent.
/// Only a trusted media processor may call this: a seal attests to processing,
/// never to camera capture. Unsigned inputs remain Unknown.
#[allow(clippy::too_many_arguments)]
pub fn sign_derivative(
    original: &[u8],
    original_mime: &str,
    rendered: &[u8],
    rendered_mime: &str,
    private_pem: &str,
    cert_pem: &str,
    ca_pem: &str,
    declared_ai: bool,
) -> Result<Vec<u8>, CoreError> {
    sign_derivative_with_edits(
        original,
        original_mime,
        rendered,
        rendered_mime,
        private_pem,
        cert_pem,
        ca_pem,
        declared_ai,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn sign_derivative_with_edits(
    original: &[u8],
    original_mime: &str,
    rendered: &[u8],
    rendered_mime: &str,
    private_pem: &str,
    cert_pem: &str,
    ca_pem: &str,
    declared_ai: bool,
    edits: &[Edit],
) -> Result<Vec<u8>, CoreError> {
    let signer = callback_signer(private_pem, cert_pem, ca_pem)?;
    let mut builder = Builder::from_context(verification_context(&VerifyOptions::new(ca_pem))?);
    let mut info = ClaimGeneratorInfo::new("Social");
    info.version = Some(VERSION.into());
    builder.set_claim_generator_info(info);
    builder.set_intent(BuilderIntent::Edit);
    builder
        .add_ingredient_from_stream(
            json!({"title": "Uploaded source", "relationship": "parentOf"}).to_string(),
            original_mime,
            &mut Cursor::new(original),
        )
        .map_err(|e| CoreError::msg(e.to_string()))?;
    let mut action = json!({
        "action": "c2pa.transcoded",
        "parameters": {"description": "Compressed by Social for delivery"}
    });
    if declared_ai {
        action["digitalSourceType"] = json!(crate::TRAINED_ALGORITHMIC);
    }
    for edit in edits {
        edit.validate()?;
        builder
            .add_action(edit.action_json())
            .map_err(|e| CoreError::msg(e.to_string()))?;
    }
    builder
        .add_action(action)
        .map_err(|e| CoreError::msg(e.to_string()))?;
    let mut dest = Cursor::new(Vec::new());
    builder
        .sign(
            &signer,
            rendered_mime,
            &mut Cursor::new(rendered),
            &mut dest,
        )
        .map_err(|e| CoreError::msg(e.to_string()))?;
    Ok(dest.into_inner())
}

/// What a media processor hands to [`seal_derivative`].
pub struct SealRequest<'a> {
    pub original: &'a [u8],
    pub original_mime: &'a str,
    pub rendered: &'a [u8],
    pub rendered_mime: &'a str,
    /// The author declared the media AI-generated: the seal carries the
    /// trained-algorithmic-media source type and the label is AI.
    pub declared_ai: bool,
    pub edits: &'a [Edit],
    /// The rendered asset has fewer pixels than the original.
    pub resized: bool,
}

/// Sign `rendered` as a derivative of `original` with the service certificate.
///
/// The manifest holds the edit actions, `c2pa.transcoded` (and `c2pa.resized`),
/// and the original as its `parentOf` ingredient. A seal attests to
/// processing, never to camera capture, and the label rules only accept it
/// when `signer` chains to the Service ICA. The caller decides whether the
/// original deserves a seal; this function does not.
///
/// A poster is sealed the same way: the video is the original, the poster
/// image is `rendered`, and `edits` holds [`Edit::Cover`].
pub fn seal_derivative(
    request: &SealRequest<'_>,
    signer: Arc<dyn DeviceSigner>,
    timestamping: &Timestamping,
    options: &VerifyOptions,
) -> Result<Vec<u8>, CoreError> {
    let err = |e: c2pa::Error| CoreError::msg(e.to_string());
    let signer = with_timestamping(c2pa_signer(signer)?, timestamping);
    let mut builder = Builder::from_context(context_for(options, true)?);
    let mut info = ClaimGeneratorInfo::new(SEAL_GENERATOR);
    info.version = Some(VERSION.into());
    builder.set_claim_generator_info(info);
    builder.set_intent(BuilderIntent::Edit);
    builder
        .add_ingredient_from_stream(
            json!({"title": "Uploaded source", "relationship": "parentOf"}).to_string(),
            request.original_mime,
            &mut Cursor::new(request.original),
        )
        .map_err(err)?;
    for edit in request.edits {
        edit.validate()?;
        builder.add_action(edit.action_json()).map_err(err)?;
    }
    let mut transcoded = json!({
        "action": "c2pa.transcoded",
        "parameters": {"description": "Compressed by Delta Sync for delivery"}
    });
    if request.declared_ai {
        transcoded["digitalSourceType"] = json!(crate::TRAINED_ALGORITHMIC);
    }
    builder.add_action(transcoded).map_err(err)?;
    if request.resized {
        builder
            .add_action(json!({
                "action": "c2pa.resized",
                "parameters": {"description": "Reduced resolution for delivery"}
            }))
            .map_err(err)?;
    }
    let mut dest = Cursor::new(Vec::new());
    builder
        .sign(
            signer.as_ref(),
            request.rendered_mime,
            &mut Cursor::new(request.rendered),
            &mut dest,
        )
        .map_err(err)?;
    Ok(dest.into_inner())
}

pub fn verify_bytes(
    asset: &[u8],
    mime: &str,
    ca_pem: &str,
    self_declared_ai: bool,
) -> LabelVerdict {
    verify_bytes_with(asset, mime, &VerifyOptions::new(ca_pem), self_declared_ai)
}

pub fn verify_bytes_with(
    asset: &[u8],
    mime: &str,
    options: &VerifyOptions,
    self_declared_ai: bool,
) -> LabelVerdict {
    verdict_from(read_report(asset, mime, options, self_declared_ai))
}

/// Verify a file on disk without loading it into memory.
/// `mime` is inferred from the extension when `None`.
pub fn verify_file(
    path: &Path,
    mime: Option<&str>,
    ca_pem: &str,
    self_declared_ai: bool,
) -> LabelVerdict {
    verify_file_with(path, mime, &VerifyOptions::new(ca_pem), self_declared_ai)
}

pub fn verify_file_with(
    path: &Path,
    mime: Option<&str>,
    options: &VerifyOptions,
    self_declared_ai: bool,
) -> LabelVerdict {
    let result = mime_for(path, mime).and_then(|mime| {
        let file = File::open(path).map_err(|e| CoreError::msg(format!("open input: {e}")))?;
        read_report_stream(file, &mime, options, self_declared_ai)
    });
    verdict_from(result)
}

fn verdict_from(result: Result<label::CredentialReport, CoreError>) -> LabelVerdict {
    match result {
        Ok(report) => {
            let mut verdict = label::assign_label(&report);
            verdict.trusted = report.embedded
                && !report.validation_codes.iter().any(|code| {
                    let code = code.to_lowercase();
                    code.contains("untrusted")
                        || code.contains("nottrusted")
                        || code.contains("mismatch")
                        || code.contains("revoked")
                        || code.contains("expired")
                });
            verdict
        }
        Err(err) => LabelVerdict {
            trusted: false,
            label: "Unknown".into(),
            summary: "Unknown · credentials could not be read".into(),
            reasons: vec!["manifest_unreadable".into(), err.to_string()],
            actions: vec![],
            signers: vec![],
        },
    }
}

/// Sign a capture on disk with a platform-held key, streaming from `in_path`
/// to `out_path`. Neither the asset nor the signed output is held in memory,
/// so a long video costs the same as a short one.
///
/// Supports JPEG, QuickTime (`.mov`) and MP4, chosen by the extension of
/// `in_path`. `out_path` must differ from `in_path` and is removed on failure.
pub fn sign_capture_file(
    in_path: &Path,
    out_path: &Path,
    signer: Arc<dyn DeviceSigner>,
) -> Result<(), CoreError> {
    sign_capture_file_with(in_path, out_path, signer, &Timestamping::None)
}

/// [`sign_capture_file`] with an RFC 3161 time stamp on the signature, so the
/// capture still verifies after the short-lived device certificate expires.
pub fn sign_capture_file_with(
    in_path: &Path,
    out_path: &Path,
    signer: Arc<dyn DeviceSigner>,
    timestamping: &Timestamping,
) -> Result<(), CoreError> {
    let mime = mime_for(in_path, None)?;
    if same_file(in_path, out_path) {
        return Err(CoreError::msg("out_path must differ from in_path"));
    }
    let result = sign_capture_file_inner(in_path, out_path, &mime, signer, timestamping);
    if result.is_err() {
        let _ = fs::remove_file(out_path);
    }
    result
}

fn sign_capture_file_inner(
    in_path: &Path,
    out_path: &Path,
    mime: &str,
    signer: Arc<dyn DeviceSigner>,
    timestamping: &Timestamping,
) -> Result<(), CoreError> {
    let signer = with_timestamping(c2pa_signer(signer)?, timestamping);
    let mut builder = Builder::from_context(signing_context()?);
    let mut info = ClaimGeneratorInfo::new(GENERATOR_NAME);
    info.version = Some(VERSION.into());
    builder.set_claim_generator_info(info);
    builder.set_intent(BuilderIntent::Create(DigitalSourceType::DigitalCapture));
    let mut source = File::open(in_path).map_err(|e| CoreError::msg(format!("open input: {e}")))?;
    let mut dest = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)
        .map_err(|e| CoreError::msg(format!("create output: {e}")))?;
    builder
        .sign(signer.as_ref(), mime, &mut source, &mut dest)
        .map_err(|e| CoreError::msg(e.to_string()))?;
    Ok(())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// MIME type for the formats the file APIs accept.
fn mime_for(path: &Path, explicit: Option<&str>) -> Result<String, CoreError> {
    if let Some(mime) = explicit {
        return Ok(mime.to_owned());
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => Ok("image/jpeg".into()),
        "mov" | "qt" => Ok("video/quicktime".into()),
        "mp4" | "m4v" => Ok("video/mp4".into()),
        other => Err(CoreError::msg(format!(
            "unsupported file extension '{other}': expected jpg, jpeg, mov or mp4"
        ))),
    }
}

/// Files above this size are staged on disk, not in memory, while signing.
const FILE_BACKING_STORE_MB: u32 = 4;

/// Context used to sign: no trust list, because the signer is the device.
/// Trust is decided by the verifier against the authority's CA.
fn signing_context() -> Result<Context, CoreError> {
    let settings = json!({
        "core": {
            // Past this size c2pa stages the intermediate copy in a temp file
            // instead of RAM, so memory stays flat for long videos.
            "backing_store_memory_threshold_in_mb": FILE_BACKING_STORE_MB,
            "merkle_tree_chunk_size_in_kb": MERKLE_CHUNK_KB
        },
        // The device accepts its own certificate profile: claim signing.
        "trust": { "trust_config": CLAIM_SIGNING_EKU_OID },
        // Trust is the verifier's call, not the device's.
        "verify": {
            "verify_trust": false,
            "verify_timestamp_trust": false,
            "remote_manifest_fetch": false
        }
    });
    Context::new()
        .with_settings(settings.to_string())
        .map_err(|e| CoreError::msg(e.to_string()))
}

fn sign(
    asset: &[u8],
    mime: &str,
    private_pem: &str,
    cert_pem: &str,
    ca_pem: &str,
    edit: Option<&Edit>,
) -> Result<Vec<u8>, CoreError> {
    let signer = callback_signer(private_pem, cert_pem, ca_pem)?;
    let mut builder = Builder::from_context(verification_context(&VerifyOptions::new(ca_pem))?);
    let mut info = ClaimGeneratorInfo::new(GENERATOR_NAME);
    info.version = Some(VERSION.into());
    builder.set_claim_generator_info(info);
    match edit {
        None => {
            builder.set_intent(BuilderIntent::Create(DigitalSourceType::DigitalCapture));
        }
        Some(edit) => {
            builder.set_intent(BuilderIntent::Edit);
            builder
                .add_action(edit.action_json())
                .map_err(|e| CoreError::msg(e.to_string()))?;
        }
    }
    let mut source = Cursor::new(asset.to_vec());
    let mut dest = Cursor::new(Vec::new());
    builder
        .sign(&signer, mime, &mut source, &mut dest)
        .map_err(|e| CoreError::msg(e.to_string()))?;
    Ok(dest.into_inner())
}

fn callback_signer(
    private_pem: &str,
    cert_pem: &str,
    ca_pem: &str,
) -> Result<CallbackSigner, CoreError> {
    let key = KeyPair::from_pem(private_pem).map_err(|e| CoreError::msg(e.to_string()))?;
    let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(key.serialized_der())
        .map_err(|e| CoreError::msg(e.to_string()))?;
    let chain = format!("{cert_pem}\n{ca_pem}");
    let callback = move |_ctx: *const (), data: &[u8]| {
        let signature = signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    };
    Ok(CallbackSigner::new(
        callback,
        SigningAlg::Ed25519,
        chain.into_bytes(),
    ))
}

fn read_report(
    asset: &[u8],
    mime: &str,
    options: &VerifyOptions,
    self_declared_ai: bool,
) -> Result<label::CredentialReport, CoreError> {
    read_report_stream(Cursor::new(asset), mime, options, self_declared_ai)
}

fn read_report_stream<R: Read + Seek + Send>(
    mut stream: R,
    mime: &str,
    options: &VerifyOptions,
    self_declared_ai: bool,
) -> Result<label::CredentialReport, CoreError> {
    let context = verification_context(options)?;
    match Reader::from_context(context).with_stream(mime, &mut stream) {
        Ok(reader) => {
            let store: serde_json::Value =
                serde_json::from_str(&reader.json()).map_err(|e| CoreError::msg(e.to_string()))?;
            let roles: HashMap<String, SignerRole> = reader
                .manifests()
                .iter()
                .map(|(id, manifest)| {
                    let role = manifest
                        .signature_info()
                        .map(|info| role_of_chain(info.cert_chain()))
                        .unwrap_or_default();
                    (id.clone(), role)
                })
                .collect();
            let mut report = label::report_from_store_with_roles(&store, self_declared_ai, &roles);
            if reader.validation_state() != c2pa::ValidationState::Trusted {
                report.validation_codes.push("validation.notTrusted".into());
            }
            Ok(report)
        }
        Err(err) => {
            let text = err.to_string().to_lowercase();
            if text.contains("no claim")
                || text.contains("not found")
                || text.contains("jumbf")
                || text.contains("no manifest")
            {
                Ok(label::CredentialReport {
                    self_declared_ai,
                    ..label::CredentialReport::default()
                })
            } else {
                Err(CoreError::msg(err.to_string()))
            }
        }
    }
}

/// The role of a signature, from the ICA that issued its leaf certificate.
///
/// The chain is the one c2pa validated against the root, so the ICA in it is
/// the root's own: only the root can sign a certificate named like one of
/// ours. A chain without exactly one of our two ICAs is `Unknown`.
fn role_of_chain(chain_pem: &str) -> SignerRole {
    use x509_parser::pem::Pem;

    let certs: Vec<Pem> = Pem::iter_from_buffer(chain_pem.as_bytes())
        .filter_map(Result::ok)
        .collect();
    let parsed: Vec<_> = certs.iter().filter_map(|p| p.parse_x509().ok()).collect();
    let Some(leaf) = parsed.first() else {
        return SignerRole::Unknown;
    };
    let issuer = parsed
        .iter()
        .skip(1)
        .find(|candidate| candidate.subject().as_raw() == leaf.issuer().as_raw());
    let common_name = issuer.and_then(|ica| {
        ica.subject()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
    });
    match common_name {
        Some(DEVICE_ICA_CN) => SignerRole::Device,
        Some(SERVICE_ICA_CN) => SignerRole::Service,
        _ => SignerRole::Unknown,
    }
}

fn verification_context(options: &VerifyOptions) -> Result<Context, CoreError> {
    context_for(options, false)
}

/// A context that trusts `options`. With `merkle`, BMFF media (MP4, MOV) is
/// hashed with a Merkle tree over its payload so it can be checked by range.
fn context_for(options: &VerifyOptions, merkle: bool) -> Result<Context, CoreError> {
    let mut anchors = vec![json!({
        "trust_anchors": options.ca_pem,
        "trust_kind": "manifest"
    })];
    if let Some(tsa) = &options.tsa_anchors_pem {
        anchors.push(json!({ "trust_anchors": tsa, "trust_kind": "tsa" }));
    }
    let mut settings = json!({
        "trust": {
            // The verifier only accepts the EKUs it is told about.
            // c2pa-kp-claimSigning is what the Delta Sync authority issues.
            "trust_config": CLAIM_SIGNING_EKU_OID,
            "anchors": anchors
        },
        "verify": { "verify_after_reading": true, "verify_trust": true, "remote_manifest_fetch": false }
    });
    if merkle {
        settings["core"] = json!({ "merkle_tree_chunk_size_in_kb": MERKLE_CHUNK_KB });
    }
    Context::new()
        .with_settings(settings.to_string())
        .map_err(|e| CoreError::msg(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{generate_device_key, DevAuthority};

    fn png() -> Vec<u8> {
        vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 32, 0, 0, 0, 24,
            8, 2, 0, 0, 0, 20, 49, 104, 99, 0, 0, 0, 36, 73, 68, 65, 84, 120, 156, 99, 120, 180,
            49, 138, 166, 136, 97, 212, 130, 81, 11, 70, 45, 24, 181, 96, 212, 130, 81, 11, 70, 45,
            24, 181, 96, 104, 88, 0, 0, 255, 65, 199, 76, 245, 107, 255, 18, 0, 0, 0, 0, 73, 69,
            78, 68, 174, 66, 96, 130,
        ]
    }

    struct Party {
        key: crate::keys::DeviceKey,
        cert: crate::keys::IssuedCert,
    }

    fn device_of(authority: &DevAuthority) -> Party {
        let key = generate_device_key().unwrap();
        let cert = authority.issue(&key.public_pem, "camera").unwrap();
        Party { key, cert }
    }

    fn service_of(authority: &DevAuthority) -> Party {
        let key = generate_device_key().unwrap();
        let cert = authority.issue_service(&key.public_pem, "media").unwrap();
        Party { key, cert }
    }

    fn capture_by(party: &Party) -> Vec<u8> {
        sign_capture(
            &png(),
            "image/png",
            &party.key.private_pem,
            &party.cert.certificate_pem,
            &party.cert.ca_pem,
        )
        .unwrap()
    }

    fn seal_by(party: &Party, source: &[u8], declared_ai: bool) -> Vec<u8> {
        sign_derivative(
            source,
            "image/png",
            &png(),
            "image/png",
            &party.key.private_pem,
            &party.cert.certificate_pem,
            &party.cert.ca_pem,
            declared_ai,
        )
        .unwrap()
    }

    #[test]
    fn a_capture_signed_under_the_device_ica_is_hm() {
        let authority = DevAuthority::generate().unwrap();
        let device = device_of(&authority);
        let verdict = verify_bytes(&capture_by(&device), "image/png", authority.ca_pem(), false);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert!(verdict.trusted, "{verdict:?}");
        assert_eq!(verdict.signers, vec!["device"]);
    }

    #[test]
    fn an_edit_signed_by_a_device_is_not_hm() {
        let authority = DevAuthority::generate().unwrap();
        let device = device_of(&authority);
        let cropped = sign_edit(
            &capture_by(&device),
            "image/png",
            &device.key.private_pem,
            &device.cert.certificate_pem,
            &device.cert.ca_pem,
            &Edit::Crop {
                x: 0,
                y: 0,
                width: 16,
                height: 12,
            },
        )
        .unwrap();
        let verdict = verify_bytes(&cropped, "image/png", authority.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(
            verdict.reasons.contains(&"device_cannot_edit".to_string()),
            "{verdict:?}"
        );
    }

    #[test]
    fn plain_png_is_unknown() {
        let authority = DevAuthority::generate().unwrap();
        let verdict = verify_bytes(&png(), "image/png", authority.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown");
    }

    #[test]
    fn a_service_seal_over_a_device_capture_is_hm_and_chains_both_icas() {
        let authority = DevAuthority::generate().unwrap();
        let captured = capture_by(&device_of(&authority));
        let sealed = seal_by(&service_of(&authority), &captured, false);
        let verdict = verify_bytes(&sealed, "image/png", authority.ca_pem(), false);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert!(verdict.trusted, "{verdict:?}");
        assert_eq!(verdict.signers, vec!["service", "device"]);
        assert!(verdict.actions.contains(&"c2pa.transcoded".into()));
    }

    #[test]
    fn a_device_certificate_cannot_produce_a_seal() {
        let authority = DevAuthority::generate().unwrap();
        let device = device_of(&authority);
        let captured = capture_by(&device);
        // The compromised device signs a "seal" itself.
        let forged = seal_by(&device, &captured, false);
        let verdict = verify_bytes(&forged, "image/png", authority.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(
            verdict.reasons.contains(&"device_cannot_edit".to_string()),
            "{verdict:?}"
        );
        // It is still a valid, trusted signature: the rule is the role.
        assert!(verdict.trusted, "{verdict:?}");
    }

    #[test]
    fn the_service_certificate_cannot_produce_a_capture() {
        let authority = DevAuthority::generate().unwrap();
        let service = service_of(&authority);
        let fabricated = capture_by(&service);
        let verdict = verify_bytes(&fabricated, "image/png", authority.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(
            verdict
                .reasons
                .contains(&"service_cannot_capture".to_string()),
            "{verdict:?}"
        );
        assert_eq!(verdict.signers, vec!["service"]);
        // Sealing over a fabricated capture does not repair it.
        let sealed = seal_by(&service, &fabricated, false);
        assert_eq!(
            verify_bytes(&sealed, "image/png", authority.ca_pem(), false).label,
            "Unknown"
        );
    }

    #[test]
    fn a_seal_never_promotes_an_unsigned_or_untrusted_source() {
        let authority = DevAuthority::generate().unwrap();
        let service = service_of(&authority);
        let captured = capture_by(&device_of(&authority));
        for (source, declared, expected) in [
            (&captured, false, "HM"),
            (&png(), false, "Unknown"),
            (&png(), true, "AI"),
        ] {
            let sealed = seal_by(&service, source, declared);
            let verdict = verify_bytes(&sealed, "image/png", authority.ca_pem(), false);
            assert_eq!(verdict.label, expected, "{verdict:?}");
        }
        // The same capture against another authority's root is untrusted.
        let other = DevAuthority::generate().unwrap();
        let verdict = verify_bytes(&captured, "image/png", other.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");

        // A capture from another authority, sealed by ours, stays Unknown.
        let foreign = capture_by(&device_of(&other));
        let sealed = seal_by(&service, &foreign, false);
        assert_eq!(
            verify_bytes(&sealed, "image/png", authority.ca_pem(), false).label,
            "Unknown",
            "a trusted processing seal must not launder an untrusted capture"
        );
    }

    #[test]
    fn an_authority_that_borrows_our_ica_names_is_not_trusted() {
        // Same names, different root: the chain does not reach our anchor.
        let ours = DevAuthority::generate().unwrap();
        let impostor = DevAuthority::generate().unwrap();
        let forged = capture_by(&device_of(&impostor));
        let verdict = verify_bytes(&forged, "image/png", ours.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(!verdict.trusted, "{verdict:?}");
    }
}
