use wasm_bindgen::prelude::*;

use crate::edit::Edit;
use crate::label::{self, CredentialReport};

#[wasm_bindgen]
pub fn core_version() -> String {
    crate::VERSION.to_string()
}

/// Describe one edit. Signing stays in the native library in v1.
#[wasm_bindgen]
pub fn describe_edit(edit_json: &str) -> String {
    let described = (|| {
        let edit: Edit = serde_json::from_str(edit_json).map_err(|e| e.to_string())?;
        edit.validate().map_err(|e| e.to_string())?;
        Ok::<_, String>(serde_json::json!({
            "ok": true,
            "action": edit.action(),
            "operation": edit.operation(),
            "parameters": edit.parameters(),
        }))
    })();
    match described {
        Ok(value) => value.to_string(),
        Err(err) => serde_json::json!({"ok": false, "error": err}).to_string(),
    }
}

#[wasm_bindgen]
pub fn assign_label_json(report_json: &str) -> String {
    let report: CredentialReport = serde_json::from_str(report_json).unwrap_or_default();
    serde_json::to_string(&label::assign_label(&report)).unwrap_or_else(|_| "{}".into())
}
