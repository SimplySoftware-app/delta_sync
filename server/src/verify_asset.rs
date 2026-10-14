use std::io::{Cursor, Read, Seek};

use anyhow::anyhow;
use c2pa::{Context, Reader, Settings};
use serde_json::{json, Value};

pub struct Verification {
    pub manifest_found: bool,
    pub format: String,
    pub validation_state: String,
    pub active_label: Option<String>,
    pub manifest: Value,
    pub validation_results: Value,
    pub detail: Option<String>,
}

pub fn verify_bytes(bytes: &[u8], format: &str, ca_pem: &str) -> anyhow::Result<Verification> {
    let settings = Settings::new()
        .with_json(
            &json!({
                "verify": {
                    "verify_trust": true,
                    "verify_after_reading": true,
                    "remote_manifest_fetch": false
                },
                "trust": {
                    "anchors": [{
                        "trust_anchors": ca_pem,
                        "trust_kind": "manifest"
                    }]
                }
            })
            .to_string(),
        )
        .map_err(|err| anyhow!("C2PA settings: {err}"))?;
    let context = Context::new()
        .with_settings(settings)
        .map_err(|err| anyhow!("C2PA context: {err}"))?;

    let mut stream = Cursor::new(bytes.to_vec());
    match Reader::from_context(context).with_stream(format, &mut stream) {
        Ok(reader) => Ok(from_reader(reader, format)),
        Err(err) => {
            let message = err.to_string();
            if is_missing_manifest(&message) {
                Ok(Verification {
                    manifest_found: false,
                    format: format.to_string(),
                    validation_state: "Absent".into(),
                    active_label: None,
                    manifest: Value::Null,
                    validation_results: Value::Null,
                    detail: Some(message),
                })
            } else {
                Err(anyhow!(message))
            }
        }
    }
}

fn from_reader(reader: Reader, format: &str) -> Verification {
    let manifest = serde_json::from_str::<Value>(&reader.json()).unwrap_or(Value::Null);
    let validation_results = reader
        .validation_results()
        .and_then(|results| serde_json::to_value(results).ok())
        .unwrap_or(Value::Null);
    Verification {
        manifest_found: true,
        format: format.to_string(),
        validation_state: format!("{:?}", reader.validation_state()),
        active_label: reader.active_label().map(str::to_string),
        manifest,
        validation_results,
        detail: None,
    }
}

fn is_missing_manifest(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("no claim")
        || lower.contains("claim missing")
        || lower.contains("manifest not found")
        || lower.contains("jumbf not found")
        || lower.contains("no jumbf")
        || lower.contains("c2pa not found")
        || lower.contains("no manifest")
}

pub fn sniff_format(bytes: &[u8], filename: &str) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        return Some("video/mp4");
    }
    match filename.rsplit('.').next().map(str::to_ascii_lowercase) {
        Some(ref ext) if ext == "jpg" || ext == "jpeg" => Some("image/jpeg"),
        Some(ref ext) if ext == "png" => Some("image/png"),
        Some(ref ext) if ext == "webp" => Some("image/webp"),
        Some(ref ext) if ext == "mp4" || ext == "mov" => Some("video/mp4"),
        _ => None,
    }
}

pub fn read_all(reader: &mut (impl Read + Seek)) -> std::io::Result<Vec<u8>> {
    reader.rewind()?;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}
