use std::io::Cursor;

use c2pa::{
    Builder, BuilderIntent, CallbackSigner, ClaimGeneratorInfo, Context, DigitalSourceType, Reader,
    SigningAlg,
};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::Signer;
use rcgen::KeyPair;
use serde_json::json;

use crate::edit::Edit;
use crate::error::CoreError;
use crate::label::{self, LabelVerdict};
use crate::{GENERATOR_NAME, VERSION};

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
    let mut builder = Builder::from_context(verification_context(ca_pem)?);
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
            .add_action(json!({"action":edit.action(), "parameters":edit.parameters()}))
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

pub fn verify_bytes(
    asset: &[u8],
    mime: &str,
    ca_pem: &str,
    self_declared_ai: bool,
) -> LabelVerdict {
    match read_report(asset, mime, ca_pem, self_declared_ai) {
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
        },
    }
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
    let mut builder = Builder::from_context(verification_context(ca_pem)?);
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
                .add_action(json!({
                    "action": edit.action(),
                    "parameters": edit.parameters(),
                }))
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
    ca_pem: &str,
    self_declared_ai: bool,
) -> Result<label::CredentialReport, CoreError> {
    let context = verification_context(ca_pem)?;
    let mut stream = Cursor::new(asset.to_vec());
    match Reader::from_context(context).with_stream(mime, &mut stream) {
        Ok(reader) => {
            let store: serde_json::Value =
                serde_json::from_str(&reader.json()).map_err(|e| CoreError::msg(e.to_string()))?;
            let mut report = label::report_from_store(&store, self_declared_ai);
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

fn verification_context(ca_pem: &str) -> Result<Context, CoreError> {
    let settings = json!({
        "trust": {
            // The verifier only accepts the EKUs it is told about.
            // c2pa-kp-claimSigning is what the Delta Sync authority issues.
            "trust_config": CLAIM_SIGNING_EKU_OID,
            "anchors": [{
                "trust_anchors": ca_pem,
                "trust_kind": "manifest"
            }]
        },
        "verify": { "verify_after_reading": true, "verify_trust": true, "remote_manifest_fetch": false }
    });
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

    #[test]
    fn capture_then_crop_is_hm() {
        let authority = DevAuthority::generate().unwrap();
        let device = generate_device_key().unwrap();
        let issued = authority.issue(&device.public_pem, "test-device").unwrap();
        let captured = sign_capture(
            &png(),
            "image/png",
            &device.private_pem,
            &issued.certificate_pem,
            &issued.ca_pem,
        )
        .unwrap();
        let cropped = sign_edit(
            &captured,
            "image/png",
            &device.private_pem,
            &issued.certificate_pem,
            &issued.ca_pem,
            &Edit::Crop {
                x: 0,
                y: 0,
                width: 16,
                height: 12,
            },
        )
        .unwrap();
        let verdict = verify_bytes(&cropped, "image/png", &issued.ca_pem, false);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert!(
            verdict.summary.contains("captured, then edited: crop"),
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
    fn seal_preserves_capture_chain_without_claiming_capture_for_unsigned_sources() {
        let authority = DevAuthority::generate().unwrap();
        let device = generate_device_key().unwrap();
        let issued = authority.issue(&device.public_pem, "camera").unwrap();
        let processor = generate_device_key().unwrap();
        let service = authority
            .issue(&processor.public_pem, "social-media")
            .unwrap();
        let captured = sign_capture(
            &png(),
            "image/png",
            &device.private_pem,
            &issued.certificate_pem,
            &issued.ca_pem,
        )
        .unwrap();
        for (source, declared, expected) in [
            (&captured, false, "HM"),
            (&png(), false, "Unknown"),
            (&png(), true, "AI"),
        ] {
            let sealed = sign_derivative(
                source,
                "image/png",
                &png(),
                "image/png",
                &processor.private_pem,
                &service.certificate_pem,
                &service.ca_pem,
                declared,
            )
            .unwrap();
            let verdict = verify_bytes(&sealed, "image/png", &service.ca_pem, false);
            assert_eq!(verdict.label, expected, "{verdict:?}");
            assert!(verdict.actions.contains(&"c2pa.transcoded".into()));
        }
        let other_authority = DevAuthority::generate().unwrap();
        let verdict = verify_bytes(&captured, "image/png", other_authority.ca_pem(), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        let other_key = generate_device_key().unwrap();
        let other_cert = other_authority
            .issue(&other_key.public_pem, "untrusted")
            .unwrap();
        let untrusted_source = sign_capture(
            &png(),
            "image/png",
            &other_key.private_pem,
            &other_cert.certificate_pem,
            &other_cert.ca_pem,
        )
        .unwrap();
        let sealed = sign_derivative(
            &untrusted_source,
            "image/png",
            &png(),
            "image/png",
            &processor.private_pem,
            &service.certificate_pem,
            &service.ca_pem,
            false,
        )
        .unwrap();
        assert_eq!(
            verify_bytes(&sealed, "image/png", &service.ca_pem, false).label,
            "Unknown",
            "a trusted processing seal must not launder an untrusted capture"
        );
        let restored =
            DevAuthority::from_pem(&authority.private_pem(), authority.ca_pem()).unwrap();
        assert_eq!(
            verify_bytes(&captured, "image/png", restored.ca_pem(), false).label,
            "HM"
        );
    }
}
