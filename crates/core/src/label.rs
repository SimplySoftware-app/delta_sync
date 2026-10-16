use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::edit::{AUDIO_REMOVED, COVER_SELECTED};
use crate::{COMPOSITE_TRAINED, GENERATOR_NAME, TRAINED_ALGORITHMIC};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Label {
    #[serde(rename = "HM")]
    Hm,
    #[serde(rename = "AI")]
    Ai,
    Unknown,
}

impl Label {
    pub fn as_str(self) -> &'static str {
        match self {
            Label::Hm => "HM",
            Label::Ai => "AI",
            Label::Unknown => "Unknown",
        }
    }
}

/// Which of our authorities signed a manifest, read from its certificate chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignerRole {
    /// The chain goes through the Device ICA: a capture.
    Device,
    /// The chain goes through the Service ICA: a seal of processing.
    Service,
    /// A third-party camera whose capture origin the official C2PA lists
    /// accept (`external_trust`). Only [`external_manifest_record`] assigns
    /// it from the native path, always with [`ManifestRecord::external`]
    /// set; without it the manifest is treated as not ours.
    External,
    /// Anything else. Never part of an HM.
    #[default]
    Unknown,
}

impl SignerRole {
    pub fn as_str(self) -> &'static str {
        match self {
            SignerRole::Device => "device",
            SignerRole::Service => "service",
            SignerRole::External => "external",
            SignerRole::Unknown => "unknown",
        }
    }
}

/// One action of a manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
}

/// One manifest of the chain (the active one and every ingredient's), with the
/// authority that signed it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRecord {
    pub id: String,
    pub role: SignerRole,
    pub actions: Vec<ActionRecord>,
    /// The accepted external camera, for [`SignerRole::External`] only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external: Option<ExternalOrigin>,
}

/// The conformance product an external capture was accepted as.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalOrigin {
    pub organization: String,
    pub product: String,
    /// `TEName` of the Trust List entity that issued the signer.
    pub issuer: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CredentialReport {
    pub embedded: bool,
    pub generator: String,
    pub issuer: String,
    pub actions: Vec<String>,
    pub operations: Vec<String>,
    pub sources: Vec<String>,
    pub validation_codes: Vec<String>,
    pub self_declared_ai: bool,
    /// Every manifest in the chain, each with the role that signed it. Without
    /// them the signer cannot be identified and the label is never HM.
    #[serde(default)]
    pub manifests: Vec<ManifestRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelVerdict {
    #[serde(default)]
    pub trusted: bool,
    pub label: String,
    pub summary: String,
    pub reasons: Vec<String>,
    pub actions: Vec<String>,
    /// Role of the signer of each manifest in the chain, newest first
    /// ("device", "service" or "unknown").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signers: Vec<String>,
}

/// What the Service ICA may sign. `c2pa.edited` is too generic to allow on its
/// own: it counts only with one of the descriptions in [`PERMITTED_EDITED`].
const PERMITTED_SERVICE: &[&str] = &[
    "c2pa.opened",
    "c2pa.cropped",
    "c2pa.orientation",
    "c2pa.trimmed",
    "c2pa.drawing",
    "c2pa.filtered",
    "c2pa.color_adjustments",
    "c2pa.transcoded",
    "c2pa.resized",
    "c2pa.published",
];

/// `c2pa.edited` descriptions the label rules accept.
const PERMITTED_EDITED: &[&str] = &[AUDIO_REMOVED, COVER_SELECTED];

fn service_may_sign(action: &ActionRecord) -> bool {
    if action.action == "c2pa.edited" {
        return PERMITTED_EDITED.contains(&action.description.as_str());
    }
    PERMITTED_SERVICE.contains(&action.action.as_str())
}

fn is_capture(action: &ActionRecord) -> bool {
    action.action == "c2pa.created" && action.source.contains("digitalCapture")
}

pub fn assign_label(report: &CredentialReport) -> LabelVerdict {
    let mut reasons = Vec::new();
    if report.self_declared_ai {
        reasons.push("self_declared_ai".into());
        return verdict(Label::Ai, "AI · declared by the author", reasons, report);
    }
    if report.sources.iter().any(|s| is_ai_source(s)) {
        reasons.push("trained_algorithmic_media".into());
        return verdict(Label::Ai, "AI · trained algorithmic media", reasons, report);
    }
    if !report.embedded {
        reasons.push("no_manifest".into());
        return verdict(
            Label::Unknown,
            "Unknown · no Content Credentials",
            reasons,
            report,
        );
    }

    let hard_fail = report.validation_codes.iter().any(|c| {
        let c = c.to_ascii_lowercase();
        c.contains("mismatch")
            || c.contains("revoked")
            || c.contains("expired")
            || c.contains("untrusted")
            || c.contains("nottrusted")
    });
    if hard_fail {
        reasons.push("validation_failed".into());
    }

    // Who may sign what. A capture is signed under the Device ICA and nothing
    // else is. Processing (edits, transcoding, sealing) is signed under the
    // Service ICA and it never claims a capture. A manifest signed by anyone
    // else, ingredients included, keeps the whole chain out of HM.
    //
    // Rule 6 for external cameras: a manifest accepted by `external_trust`
    // may stand in for the device capture. Its own further actions, and every
    // later manifest, must be permitted actions sealed by our Service ICA.
    // Our devices never sign over it and there is only one capture origin.
    if report.manifests.is_empty() {
        reasons.push("signer_not_identified".into());
    }
    for manifest in &report.manifests {
        match manifest.role {
            SignerRole::Unknown => push_once(&mut reasons, "signer_not_ours"),
            SignerRole::External if manifest.external.is_none() => {
                push_once(&mut reasons, "signer_not_ours")
            }
            SignerRole::External => {
                for action in &manifest.actions {
                    if action.action == "c2pa.created" {
                        if !is_capture(action) {
                            push_once(&mut reasons, "not_digital_capture");
                        }
                    } else if !service_may_sign(action) {
                        push_once(&mut reasons, "disallowed_action");
                    }
                }
            }
            SignerRole::Device => {
                for action in &manifest.actions {
                    if action.action == "c2pa.created" {
                        if !is_capture(action) {
                            push_once(&mut reasons, "not_digital_capture");
                        }
                    } else if action.action != "c2pa.opened" {
                        push_once(&mut reasons, "device_cannot_edit");
                    }
                }
            }
            SignerRole::Service => {
                for action in &manifest.actions {
                    if action.action == "c2pa.created" || action.source.contains("digitalCapture") {
                        push_once(&mut reasons, "service_cannot_capture");
                    } else if !service_may_sign(action) {
                        push_once(&mut reasons, "disallowed_action");
                    }
                }
            }
        }
    }
    let device_captures = report
        .manifests
        .iter()
        .filter(|m| m.role == SignerRole::Device && m.actions.iter().any(is_capture))
        .count();
    let external_captures: Vec<&ExternalOrigin> = report
        .manifests
        .iter()
        .filter(|m| m.role == SignerRole::External && m.actions.iter().any(is_capture))
        .filter_map(|m| m.external.as_ref())
        .collect();
    if external_captures.is_empty() {
        if device_captures == 0 {
            push_once(&mut reasons, "no_device_capture");
        }
    } else {
        if external_captures.len() + device_captures > 1 {
            push_once(&mut reasons, "multiple_capture_origins");
        }
        if report
            .manifests
            .iter()
            .any(|m| m.role == SignerRole::Device)
        {
            push_once(&mut reasons, "device_over_external");
        }
    }

    let rules_ok = !hard_fail && reasons.is_empty();
    if rules_ok {
        let edits: Vec<&str> = report
            .operations
            .iter()
            .filter(|op| !op.is_empty() && *op != "capture")
            .map(String::as_str)
            .collect();
        let captured = match external_captures.first() {
            Some(origin) => format!("captured with {} ({})", origin.product, origin.organization),
            None => "captured".to_string(),
        };
        let summary = if edits.is_empty() {
            format!("HM · {captured}")
        } else {
            format!("HM · {captured}, then edited: {}", edits.join(", "))
        };
        reasons.push(if external_captures.is_empty() {
            "delta_sync_capture".into()
        } else {
            "external_capture".into()
        });
        return verdict(Label::Hm, summary, reasons, report);
    }

    verdict(
        Label::Unknown,
        "Unknown · credentials were not accepted as HM",
        reasons,
        report,
    )
}

fn push_once(reasons: &mut Vec<String>, reason: &str) {
    if !reasons.iter().any(|r| r == reason) {
        reasons.push(reason.to_string());
    }
}

fn verdict(
    label: Label,
    summary: impl Into<String>,
    reasons: Vec<String>,
    report: &CredentialReport,
) -> LabelVerdict {
    LabelVerdict {
        trusted: false,
        label: label.as_str().into(),
        summary: summary.into(),
        reasons,
        actions: report.actions.clone(),
        signers: report
            .manifests
            .iter()
            .map(|m| m.role.as_str().to_string())
            .collect(),
    }
}

fn is_ai_source(value: &str) -> bool {
    value.contains("trainedAlgorithmicMedia")
        || value.contains(TRAINED_ALGORITHMIC)
        || value.contains(COMPOSITE_TRAINED)
        || value.contains("compositeWithTrainedAlgorithmicMedia")
}

/// Walk a C2PA manifest-store JSON document into a [`CredentialReport`].
/// `roles` maps a manifest label to the authority that signed it.
pub fn report_from_store_with_roles(
    store: &Value,
    self_declared_ai: bool,
    roles: &HashMap<String, SignerRole>,
) -> CredentialReport {
    let mut report = CredentialReport {
        embedded: true,
        self_declared_ai,
        ..CredentialReport::default()
    };
    if let Some(codes) = store.get("validation_status").and_then(Value::as_array) {
        for code in codes {
            if let Some(c) = code.get("code").and_then(Value::as_str) {
                if !is_ignorable_code(c) {
                    report.validation_codes.push(c.to_string());
                }
            }
        }
    }
    walk(store, &mut report);
    if let Some(manifests) = store.get("manifests").and_then(Value::as_object) {
        let active = store.get("active_manifest").and_then(Value::as_str);
        // Newest first: the active manifest, then the rest in a stable order.
        let mut ids: Vec<&String> = manifests.keys().collect();
        ids.sort_by_key(|id| (Some(id.as_str()) != active, id.to_string()));
        for id in ids {
            report.manifests.push(ManifestRecord {
                id: id.clone(),
                // External needs a verdict; a role map cannot grant it.
                role: match roles.get(id).copied().unwrap_or_default() {
                    SignerRole::External => SignerRole::Unknown,
                    role => role,
                },
                actions: actions_of(&manifests[id]),
                external: None,
            });
        }
    }
    if report.generator.is_empty() {
        if let Some(active) = store.get("active_manifest").and_then(Value::as_str) {
            if let Some(gen) = store
                .pointer(&format!("/manifests/{active}/claim_generator"))
                .and_then(Value::as_str)
            {
                report.generator = gen.to_string();
            }
        }
    }
    report
}

fn walk(value: &Value, report: &mut CredentialReport) {
    match value {
        Value::Object(map) => {
            if map
                .get("failure")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
            {
                report.validation_codes.push("validation.notTrusted".into());
            }
            // Ingredient validation is nested. A trusted active seal must not
            // hide an untrusted or altered capture further down the chain.
            if let Some(Value::String(code)) = map.get("code") {
                if !is_ignorable_code(code) && !report.validation_codes.contains(code) {
                    report.validation_codes.push(code.clone());
                }
            }
            if let Some(Value::String(action)) = map.get("action") {
                report.actions.push(action.clone());
                if let Some(op) = operation_for(action, &description_of(map)) {
                    if !report.operations.contains(&op.to_string()) {
                        report.operations.push(op.to_string());
                    }
                }
            }
            if let Some(Value::String(source)) = map.get("digitalSourceType") {
                report.sources.push(source.clone());
                if source.contains("digitalCapture")
                    && !report.operations.contains(&"capture".into())
                {
                    // capture is not an "edit" in the summary; tracked via sources
                }
            }
            if report.generator.is_empty() {
                if let Some(Value::String(name)) = map.get("name") {
                    if map.contains_key("version") || map.contains_key("specVersion") {
                        // claim_generator_info entries
                        if name == GENERATOR_NAME || name.contains(GENERATOR_NAME) {
                            report.generator = name.clone();
                        }
                    }
                }
                if let Some(Value::String(gen)) = map.get("claim_generator") {
                    report.generator = gen.clone();
                }
            }
            if report.issuer.is_empty() {
                if let Some(Value::String(issuer)) = map.get("issuer") {
                    report.issuer = issuer.clone();
                }
            }
            for child in map.values() {
                walk(child, report);
            }
        }
        Value::Array(items) => {
            for child in items {
                walk(child, report);
            }
        }
        _ => {}
    }
}

/// An RFC 3161 time-stamp is checked against a TSA trust list, and ours is not
/// on one (the authority uses a public TSA). The manifest's trust is the chain
/// to our own root, so an untrusted TSA is informational, not a failure. A
/// time-stamp that does not match its signature is still a failure.
fn is_ignorable_code(code: &str) -> bool {
    code == "timeStamp.untrusted"
}

/// The actions of one manifest, from its `c2pa.actions` assertions.
fn actions_of(manifest: &Value) -> Vec<ActionRecord> {
    let mut records = Vec::new();
    let Some(assertions) = manifest.get("assertions").and_then(Value::as_array) else {
        return records;
    };
    for assertion in assertions {
        let is_actions = assertion
            .get("label")
            .and_then(Value::as_str)
            .is_some_and(|l| l.starts_with("c2pa.actions"));
        if !is_actions {
            continue;
        }
        let Some(actions) = assertion.pointer("/data/actions").and_then(Value::as_array) else {
            continue;
        };
        for action in actions {
            let Some(name) = action.get("action").and_then(Value::as_str) else {
                continue;
            };
            records.push(ActionRecord {
                action: name.to_string(),
                description: description_of(action.as_object().unwrap_or(&Default::default())),
                source: action
                    .get("digitalSourceType")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }
    records
}

/// The `description` of an action: its own field, or the one in `parameters`.
fn description_of(action: &serde_json::Map<String, Value>) -> String {
    action
        .get("description")
        .or_else(|| action.get("parameters").and_then(|p| p.get("description")))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn operation_for(action: &str, description: &str) -> Option<&'static str> {
    match action {
        "c2pa.edited" if description == AUDIO_REMOVED => Some("mute"),
        "c2pa.edited" if description == COVER_SELECTED => Some("cover"),
        "c2pa.cropped" => Some("crop"),
        "c2pa.trimmed" => Some("trim"),
        "c2pa.orientation" => Some("rotate"),
        "c2pa.drawing" => Some("text"),
        "c2pa.filtered" => Some("filter"),
        "c2pa.transcoded" => Some("compression"),
        _ => None,
    }
}

/// The record of a manifest signed by a third-party camera, judged by the
/// official C2PA lists ([`crate::external_trust::external_capture_verdict`],
/// rules 1-5). An accepted capture origin gets [`SignerRole::External`];
/// anything else is [`SignerRole::Unknown`] and keeps the chain out of HM.
/// [`assign_label`] then applies rule 6 to whatever was signed after it.
///
/// Same contract as the verdict: `info` must come from a manifest c2pa-rs
/// has already validated, and its MIME from the asset bytes.
#[cfg(feature = "c2pa")]
pub fn external_manifest_record(
    id: &str,
    info: &crate::external_trust::ExternalManifestInfo,
) -> ManifestRecord {
    external_record_from(
        id,
        info,
        crate::external_trust::external_capture_verdict(info),
    )
}

#[cfg(feature = "c2pa")]
pub(crate) fn external_record_from(
    id: &str,
    info: &crate::external_trust::ExternalManifestInfo,
    verdict: crate::external_trust::ExternalVerdict,
) -> ManifestRecord {
    use crate::external_trust::ExternalVerdict;
    let (role, external) = match verdict {
        ExternalVerdict::Hm { product } => (
            SignerRole::External,
            Some(ExternalOrigin {
                organization: product.organization,
                product: product.common_name,
                issuer: product.issuer,
            }),
        ),
        ExternalVerdict::Unknown { .. } => (SignerRole::Unknown, None),
    };
    ManifestRecord {
        id: id.to_string(),
        role,
        actions: info
            .actions
            .iter()
            .map(|a| ActionRecord {
                action: a.action.clone(),
                description: String::new(),
                source: a.digital_source_type.clone().unwrap_or_default(),
            })
            .collect(),
        external,
    }
}

#[allow(dead_code)]
pub fn empty_report() -> CredentialReport {
    CredentialReport::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DIGITAL_CAPTURE;
    use serde_json::json;

    fn action(name: &str) -> ActionRecord {
        ActionRecord {
            action: name.into(),
            ..ActionRecord::default()
        }
    }

    fn described(name: &str, description: &str) -> ActionRecord {
        ActionRecord {
            description: description.into(),
            ..action(name)
        }
    }

    fn created() -> ActionRecord {
        ActionRecord {
            source: DIGITAL_CAPTURE.into(),
            ..action("c2pa.created")
        }
    }

    fn manifest(role: SignerRole, actions: Vec<ActionRecord>) -> ManifestRecord {
        ManifestRecord {
            id: format!("urn:{}", role.as_str()),
            role,
            actions,
            external: None,
        }
    }

    /// A report built the way the verifier builds it from a chain.
    fn report(manifests: Vec<ManifestRecord>) -> CredentialReport {
        let mut report = CredentialReport {
            embedded: true,
            generator: "Delta Sync/0.1.0".into(),
            issuer: "Delta Sync Device".into(),
            ..CredentialReport::default()
        };
        for manifest in &manifests {
            for action in &manifest.actions {
                report.actions.push(action.action.clone());
                if !action.source.is_empty() {
                    report.sources.push(action.source.clone());
                }
                if let Some(op) = operation_for(&action.action, &action.description) {
                    if !report.operations.contains(&op.to_string()) {
                        report.operations.push(op.to_string());
                    }
                }
            }
        }
        report.manifests = manifests;
        report
    }

    fn capture() -> CredentialReport {
        report(vec![manifest(SignerRole::Device, vec![created()])])
    }

    /// A capture under the Device ICA sealed under the Service ICA.
    fn sealed(seal: Vec<ActionRecord>) -> CredentialReport {
        report(vec![
            manifest(SignerRole::Service, seal),
            manifest(SignerRole::Device, vec![created()]),
        ])
    }

    fn reasons(report: &CredentialReport) -> Vec<String> {
        assign_label(report).reasons
    }

    #[test]
    fn capture_under_the_device_ica_is_hm() {
        let verdict = assign_label(&capture());
        assert_eq!(verdict.label, "HM");
        assert_eq!(verdict.summary, "HM · captured");
        assert_eq!(verdict.signers, vec!["device"]);
    }

    #[test]
    fn a_service_seal_over_a_device_capture_is_hm() {
        let report = sealed(vec![
            action("c2pa.opened"),
            action("c2pa.cropped"),
            action("c2pa.transcoded"),
            action("c2pa.resized"),
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert_eq!(
            verdict.summary,
            "HM · captured, then edited: crop, compression"
        );
        assert_eq!(verdict.signers, vec!["service", "device"]);
    }

    #[test]
    fn a_device_cannot_produce_a_seal() {
        // The device certificate signs a transcode: a compromised device
        // must not be able to issue what only the service may.
        let report = report(vec![
            manifest(
                SignerRole::Device,
                vec![action("c2pa.opened"), action("c2pa.transcoded")],
            ),
            manifest(SignerRole::Device, vec![created()]),
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(verdict.reasons.contains(&"device_cannot_edit".to_string()));
    }

    #[test]
    fn a_device_cannot_sign_an_edit_either() {
        let report = report(vec![
            manifest(
                SignerRole::Device,
                vec![action("c2pa.opened"), action("c2pa.cropped")],
            ),
            manifest(SignerRole::Device, vec![created()]),
        ]);
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn the_service_cannot_produce_a_capture() {
        let report = report(vec![manifest(SignerRole::Service, vec![created()])]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(verdict
            .reasons
            .contains(&"service_cannot_capture".to_string()));
        assert!(verdict.reasons.contains(&"no_device_capture".to_string()));
    }

    #[test]
    fn a_service_seal_cannot_smuggle_a_capture_claim_over_a_foreign_origin() {
        // A seal whose only capture claim is its own, over an unsigned origin.
        let report = report(vec![manifest(
            SignerRole::Service,
            vec![action("c2pa.opened"), created(), action("c2pa.transcoded")],
        )]);
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn any_untrusted_ingredient_makes_the_chain_unknown() {
        let report = report(vec![
            manifest(SignerRole::Service, vec![action("c2pa.transcoded")]),
            manifest(SignerRole::Unknown, vec![created()]),
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        assert!(verdict.reasons.contains(&"signer_not_ours".to_string()));

        // Even with a good capture underneath, a stray manifest signed by
        // someone else poisons the chain.
        let report = report_with(vec![
            manifest(SignerRole::Service, vec![action("c2pa.transcoded")]),
            manifest(SignerRole::Unknown, vec![action("c2pa.opened")]),
            manifest(SignerRole::Device, vec![created()]),
        ]);
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    fn report_with(manifests: Vec<ManifestRecord>) -> CredentialReport {
        report(manifests)
    }

    #[test]
    fn a_report_that_names_no_signer_is_never_hm() {
        // The generator string alone proves nothing.
        let mut report = capture();
        report.manifests.clear();
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict
            .reasons
            .contains(&"signer_not_identified".to_string()));
    }

    #[test]
    fn a_report_with_the_name_delta_sync_but_an_unknown_signer_is_unknown() {
        let report = report(vec![manifest(SignerRole::Unknown, vec![created()])]);
        assert_eq!(report.generator, "Delta Sync/0.1.0");
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn mute_is_a_permitted_edit_only_with_its_description() {
        let muted = sealed(vec![
            action("c2pa.opened"),
            described("c2pa.edited", AUDIO_REMOVED),
            action("c2pa.transcoded"),
        ]);
        let verdict = assign_label(&muted);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert_eq!(
            verdict.summary,
            "HM · captured, then edited: mute, compression"
        );

        let anything = sealed(vec![
            action("c2pa.opened"),
            described("c2pa.edited", "replaced the sky"),
            action("c2pa.transcoded"),
        ]);
        assert_eq!(assign_label(&anything).label, "Unknown");
        assert!(reasons(&anything).contains(&"disallowed_action".to_string()));

        let bare = sealed(vec![action("c2pa.opened"), action("c2pa.edited")]);
        assert_eq!(assign_label(&bare).label, "Unknown");
    }

    #[test]
    fn cover_is_a_permitted_edit() {
        let report = sealed(vec![
            action("c2pa.opened"),
            described("c2pa.edited", COVER_SELECTED),
            action("c2pa.transcoded"),
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert!(verdict.summary.contains("cover"), "{verdict:?}");
    }

    #[test]
    fn actions_outside_the_list_block_hm() {
        let report = sealed(vec![action("c2pa.opened"), action("c2pa.placed")]);
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn trained_media_is_ai_even_if_untrusted() {
        let mut report = capture();
        report.sources = vec![TRAINED_ALGORITHMIC.into()];
        assert_eq!(assign_label(&report).label, "AI");
    }

    #[test]
    fn self_declared_ai_wins() {
        let mut report = capture();
        report.self_declared_ai = true;
        assert_eq!(assign_label(&report).label, "AI");
    }

    #[test]
    fn missing_manifest_is_unknown() {
        assert_eq!(assign_label(&CredentialReport::default()).label, "Unknown");
    }

    #[test]
    fn hash_mismatch_blocks_hm() {
        let mut report = capture();
        report
            .validation_codes
            .push("assertion.dataHash.mismatch".into());
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn an_expired_certificate_blocks_hm() {
        let mut report = capture();
        report
            .validation_codes
            .push("signingCredential.expired".into());
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn untrusted_signature_cannot_claim_hm_by_naming_delta_sync() {
        let mut report = capture();
        report
            .validation_codes
            .push("signingCredential.untrusted".into());
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    // Rule 6: external cameras.

    fn origin() -> ExternalOrigin {
        ExternalOrigin {
            organization: "Google LLC".into(),
            product: "Pixel Camera".into(),
            issuer: "Google LLC".into(),
        }
    }

    fn external(actions: Vec<ActionRecord>) -> ManifestRecord {
        ManifestRecord {
            external: Some(origin()),
            ..manifest(SignerRole::External, actions)
        }
    }

    #[test]
    fn an_accepted_external_capture_is_hm() {
        let verdict = assign_label(&report(vec![external(vec![created()])]));
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert_eq!(
            verdict.summary,
            "HM · captured with Pixel Camera (Google LLC)"
        );
        assert_eq!(verdict.reasons, vec!["external_capture"]);
        assert_eq!(verdict.signers, vec!["external"]);
    }

    #[test]
    fn our_service_seal_over_an_external_capture_stays_hm() {
        let report = report(vec![
            manifest(
                SignerRole::Service,
                vec![
                    action("c2pa.opened"),
                    action("c2pa.cropped"),
                    described("c2pa.edited", AUDIO_REMOVED),
                    action("c2pa.transcoded"),
                ],
            ),
            external(vec![created()]),
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
        assert_eq!(
            verdict.summary,
            "HM · captured with Pixel Camera (Google LLC), then edited: crop, mute, compression"
        );
        assert_eq!(verdict.signers, vec!["service", "external"]);
    }

    #[test]
    fn a_non_permitted_edit_over_an_external_capture_is_unknown() {
        for bad in [
            action("c2pa.placed"),
            action("c2pa.edited"),
            described("c2pa.edited", "replaced the sky"),
        ] {
            let report = report(vec![
                manifest(
                    SignerRole::Service,
                    vec![action("c2pa.opened"), bad.clone()],
                ),
                external(vec![created()]),
            ]);
            let verdict = assign_label(&report);
            assert_eq!(verdict.label, "Unknown", "{bad:?}");
            assert!(verdict.reasons.contains(&"disallowed_action".to_string()));
        }
    }

    #[test]
    fn the_external_manifest_itself_may_only_add_permitted_actions() {
        let ok = report(vec![external(vec![created(), action("c2pa.resized")])]);
        assert_eq!(assign_label(&ok).label, "HM");
        let bad = report(vec![external(vec![created(), action("c2pa.placed")])]);
        let verdict = assign_label(&bad);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict.reasons.contains(&"disallowed_action".to_string()));
    }

    #[test]
    fn an_edit_signed_by_anyone_else_over_an_external_capture_is_unknown() {
        // A third-party editor (not our Service ICA) re-signs the capture.
        let report = report(vec![
            manifest(SignerRole::Unknown, vec![action("c2pa.opened")]),
            external(vec![created()]),
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict.reasons.contains(&"signer_not_ours".to_string()));
    }

    #[test]
    fn our_device_never_signs_over_an_external_capture() {
        let opened = report(vec![
            manifest(SignerRole::Device, vec![action("c2pa.opened")]),
            external(vec![created()]),
        ]);
        let verdict = assign_label(&opened);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict
            .reasons
            .contains(&"device_over_external".to_string()));

        let two = report(vec![
            manifest(SignerRole::Device, vec![created()]),
            external(vec![created()]),
        ]);
        let verdict = assign_label(&two);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict
            .reasons
            .contains(&"multiple_capture_origins".to_string()));
    }

    #[test]
    fn only_one_external_capture_origin() {
        let report = report(vec![
            external(vec![created()]),
            ManifestRecord {
                id: "urn:other".into(),
                ..external(vec![created()])
            },
        ]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict
            .reasons
            .contains(&"multiple_capture_origins".to_string()));
    }

    #[test]
    fn an_external_role_without_an_accepted_origin_is_not_ours() {
        let report = report(vec![manifest(SignerRole::External, vec![created()])]);
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "Unknown");
        assert!(verdict.reasons.contains(&"signer_not_ours".to_string()));
    }

    #[test]
    fn an_external_capture_still_needs_valid_credentials() {
        let mut r = report(vec![external(vec![created()])]);
        r.validation_codes
            .push("assertion.dataHash.mismatch".into());
        assert_eq!(assign_label(&r).label, "Unknown");
        let mut r = report(vec![external(vec![created()])]);
        r.sources.push(TRAINED_ALGORITHMIC.into());
        assert_eq!(assign_label(&r).label, "AI");
    }

    #[test]
    fn a_role_map_cannot_grant_the_external_role() {
        let store = store_with_actions(json!([{
            "action": "c2pa.created",
            "digitalSourceType": DIGITAL_CAPTURE
        }]));
        let roles = HashMap::from([("urn:1".to_string(), SignerRole::External)]);
        let report = report_from_store_with_roles(&store, false, &roles);
        assert_eq!(report.manifests[0].role, SignerRole::Unknown);
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[cfg(feature = "c2pa")]
    #[test]
    fn the_external_verdict_decides_the_role() {
        use crate::external_trust::{
            ExternalAction, ExternalManifestInfo, ExternalProduct, ExternalVerdict,
        };
        let info = ExternalManifestInfo {
            actions: vec![ExternalAction {
                action: "c2pa.created".into(),
                digital_source_type: Some(DIGITAL_CAPTURE.into()),
            }],
            ..ExternalManifestInfo::default()
        };
        let hm = ExternalVerdict::Hm {
            product: ExternalProduct {
                organization: "Google LLC".into(),
                common_name: "Pixel Camera".into(),
                min_version: None,
                assurance_level: 2,
                issuer: "Google LLC".into(),
            },
        };
        let record = external_record_from("urn:x", &info, hm);
        assert_eq!(record.role, SignerRole::External);
        assert_eq!(record.external, Some(origin()));
        assert_eq!(record.actions, vec![created()]);
        assert_eq!(assign_label(&report(vec![record])).label, "HM");

        let unknown = ExternalVerdict::Unknown {
            reason: "not bound".into(),
        };
        let record = external_record_from("urn:x", &info, unknown);
        assert_eq!(record.role, SignerRole::Unknown);
        assert_eq!(record.external, None);
        assert_eq!(assign_label(&report(vec![record])).label, "Unknown");

        // The production entry point with an untrusted chain.
        let record = external_manifest_record("urn:x", &info);
        assert_eq!(record.role, SignerRole::Unknown);
    }

    fn store_with_actions(actions: serde_json::Value) -> serde_json::Value {
        json!({
            "active_manifest": "urn:1",
            "validation_status": [{"code": "timeStamp.untrusted"}],
            "manifests": {"urn:1": {
                "claim_generator": "Delta Sync/0.1.0",
                "assertions": [{"label": "c2pa.actions.v2", "data": {"actions": actions}}]
            }},
            "validation_results": {"activeManifest": {"informational": [
                {"code": "timeStamp.untrusted", "explanation": "timestamp cert untrusted"}
            ]}}
        })
    }

    #[test]
    fn an_untrusted_time_stamp_authority_does_not_block_hm() {
        let store = store_with_actions(json!([{
            "action": "c2pa.created",
            "digitalSourceType": DIGITAL_CAPTURE
        }]));
        let roles = HashMap::from([("urn:1".to_string(), SignerRole::Device)]);
        let report = report_from_store_with_roles(&store, false, &roles);
        assert!(
            report.validation_codes.is_empty(),
            "{:?}",
            report.validation_codes
        );
        assert_eq!(assign_label(&report).label, "HM");
        let mut broken = report.clone();
        broken.validation_codes.push("timeStamp.mismatch".into());
        assert_eq!(assign_label(&broken).label, "Unknown");
    }

    #[test]
    fn walker_reads_actions_descriptions_and_roles() {
        let store = store_with_actions(json!([
            {"action": "c2pa.created", "digitalSourceType": DIGITAL_CAPTURE},
            {"action": "c2pa.edited", "description": AUDIO_REMOVED},
            {"action": "c2pa.edited", "parameters": {"description": COVER_SELECTED}}
        ]));
        let unknown = report_from_store_with_roles(&store, false, &HashMap::new());
        assert_eq!(unknown.manifests[0].role, SignerRole::Unknown);
        assert_eq!(assign_label(&unknown).label, "Unknown");

        let roles = HashMap::from([("urn:1".to_string(), SignerRole::Service)]);
        let report = report_from_store_with_roles(&store, false, &roles);
        assert!(report.actions.iter().any(|a| a == "c2pa.created"));
        assert!(report.sources.iter().any(|s| s.contains("digitalCapture")));
        assert!(report.generator.contains("Delta Sync"));
        assert_eq!(report.manifests.len(), 1);
        assert_eq!(report.manifests[0].role, SignerRole::Service);
        assert_eq!(report.manifests[0].actions[1].description, AUDIO_REMOVED);
        assert_eq!(report.manifests[0].actions[2].description, COVER_SELECTED);
        assert_eq!(report.operations, vec!["mute", "cover"]);
    }
}
