use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelVerdict {
    #[serde(default)]
    pub trusted: bool,
    pub label: String,
    pub summary: String,
    pub reasons: Vec<String>,
    pub actions: Vec<String>,
}

const PERMITTED: &[&str] = &[
    "c2pa.created",
    "c2pa.opened",
    "c2pa.cropped",
    "c2pa.orientation",
    "c2pa.trimmed",
    "c2pa.drawing",
    "c2pa.filtered",
    "c2pa.color_adjustments",
    "c2pa.transcoded",
    "c2pa.published",
];

pub fn assign_label(report: &CredentialReport) -> LabelVerdict {
    let mut reasons = Vec::new();
    if report.self_declared_ai {
        reasons.push("self_declared_ai".into());
        return verdict(
            Label::Ai,
            "AI · declared by the author",
            reasons,
            &report.actions,
        );
    }
    if report.sources.iter().any(|s| is_ai_source(s)) {
        reasons.push("trained_algorithmic_media".into());
        return verdict(
            Label::Ai,
            "AI · trained algorithmic media",
            reasons,
            &report.actions,
        );
    }
    if !report.embedded {
        reasons.push("no_manifest".into());
        return verdict(
            Label::Unknown,
            "Unknown · no Content Credentials",
            reasons,
            &report.actions,
        );
    }

    let ours = report.generator.contains(GENERATOR_NAME) || report.issuer.contains("Delta Sync");
    let hard_fail = report.validation_codes.iter().any(|c| {
        let c = c.to_ascii_lowercase();
        c.contains("mismatch")
            || c.contains("revoked")
            || c.contains("expired")
            || c.contains("untrusted")
            || c.contains("nottrusted")
    });
    let actions_ok = report
        .actions
        .iter()
        .all(|a| PERMITTED.contains(&a.as_str()));
    let capture = report.sources.iter().any(|s| s.contains("digitalCapture"));

    if !ours {
        reasons.push("generator_not_allowlisted".into());
    }
    if !capture {
        reasons.push("not_digital_capture".into());
    }
    if !actions_ok {
        reasons.push("disallowed_action".into());
    }
    if hard_fail {
        reasons.push("validation_failed".into());
    }
    if ours && capture && actions_ok && !hard_fail {
        let edits: Vec<&str> = report
            .operations
            .iter()
            .filter(|op| !op.is_empty() && *op != "capture")
            .map(String::as_str)
            .collect();
        let summary = if edits.is_empty() {
            "HM · captured".to_string()
        } else {
            format!("HM · captured, then edited: {}", edits.join(", "))
        };
        reasons.push("delta_sync_capture".into());
        return verdict(Label::Hm, summary, reasons, &report.actions);
    }

    verdict(
        Label::Unknown,
        "Unknown · credentials were not accepted as HM",
        reasons,
        &report.actions,
    )
}

fn verdict(
    label: Label,
    summary: impl Into<String>,
    reasons: Vec<String>,
    actions: &[String],
) -> LabelVerdict {
    LabelVerdict {
        trusted: false,
        label: label.as_str().into(),
        summary: summary.into(),
        reasons,
        actions: actions.to_vec(),
    }
}

fn is_ai_source(value: &str) -> bool {
    value.contains("trainedAlgorithmicMedia")
        || value.contains(TRAINED_ALGORITHMIC)
        || value.contains(COMPOSITE_TRAINED)
        || value.contains("compositeWithTrainedAlgorithmicMedia")
}

/// Walk a C2PA manifest-store JSON document into a [`CredentialReport`].
pub fn report_from_store(store: &Value, self_declared_ai: bool) -> CredentialReport {
    let mut report = CredentialReport {
        embedded: true,
        self_declared_ai,
        ..CredentialReport::default()
    };
    if let Some(codes) = store.get("validation_status").and_then(Value::as_array) {
        for code in codes {
            if let Some(c) = code.get("code").and_then(Value::as_str) {
                report.validation_codes.push(c.to_string());
            }
        }
    }
    walk(store, &mut report);
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
                if !report.validation_codes.contains(code) {
                    report.validation_codes.push(code.clone());
                }
            }
            if let Some(Value::String(action)) = map.get("action") {
                report.actions.push(action.clone());
                if let Some(op) = operation_for(action) {
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

fn operation_for(action: &str) -> Option<&'static str> {
    match action {
        "c2pa.cropped" => Some("crop"),
        "c2pa.trimmed" => Some("trim"),
        "c2pa.orientation" => Some("rotate"),
        "c2pa.drawing" => Some("text"),
        "c2pa.filtered" => Some("filter"),
        "c2pa.transcoded" => Some("compression"),
        _ => None,
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

    fn capture() -> CredentialReport {
        CredentialReport {
            embedded: true,
            generator: "Delta Sync/0.1.0".into(),
            issuer: "CN=Delta Sync Dev CA".into(),
            actions: vec!["c2pa.created".into()],
            operations: vec![],
            sources: vec![DIGITAL_CAPTURE.into()],
            validation_codes: vec![],
            self_declared_ai: false,
        }
    }

    #[test]
    fn capture_from_our_authority_is_hm() {
        let verdict = assign_label(&capture());
        assert_eq!(verdict.label, "HM");
        assert_eq!(verdict.summary, "HM · captured");
    }

    #[test]
    fn crop_keeps_hm() {
        let mut report = capture();
        report.actions.push("c2pa.cropped".into());
        report.operations.push("crop".into());
        let verdict = assign_label(&report);
        assert_eq!(verdict.label, "HM");
        assert_eq!(verdict.summary, "HM · captured, then edited: crop");
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
    fn untrusted_signature_cannot_claim_hm_by_naming_delta_sync() {
        let mut report = capture();
        report
            .validation_codes
            .push("signingCredential.untrusted".into());
        assert_eq!(assign_label(&report).label, "Unknown");
    }

    #[test]
    fn walker_reads_actions() {
        let store = json!({
            "active_manifest": "urn:1",
            "manifests": {
                "urn:1": {
                    "claim_generator": "Delta Sync/0.1.0 c2pa-rs/0.91.2",
                    "assertions": [{
                        "label": "c2pa.actions",
                        "data": { "actions": [{
                            "action": "c2pa.created",
                            "digitalSourceType": DIGITAL_CAPTURE
                        }]}
                    }],
                    "signature_info": { "issuer": "CN=Delta Sync Dev CA" }
                }
            }
        });
        let report = report_from_store(&store, false);
        assert!(report.actions.iter().any(|a| a == "c2pa.created"));
        assert!(report.sources.iter().any(|s| s.contains("digitalCapture")));
        assert!(report.generator.contains("Delta Sync"));
    }
}
