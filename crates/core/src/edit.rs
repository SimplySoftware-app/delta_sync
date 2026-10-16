use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::CoreError;

/// One edit the SDK can sign. Rendering stays in the host app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Trim {
        start_ms: u64,
        end_ms: u64,
    },
    Rotate {
        degrees: i32,
    },
    Text {
        body: String,
    },
    /// Stretch. The core can sign it. The apps do not show it in v1.
    Filter {
        name: String,
    },
}

impl Edit {
    pub fn action(&self) -> &'static str {
        match self {
            Edit::Crop { .. } => "c2pa.cropped",
            Edit::Trim { .. } => "c2pa.trimmed",
            Edit::Rotate { .. } => "c2pa.orientation",
            Edit::Text { .. } => "c2pa.drawing",
            Edit::Filter { .. } => "c2pa.filtered",
        }
    }

    pub fn operation(&self) -> &'static str {
        match self {
            Edit::Crop { .. } => "crop",
            Edit::Trim { .. } => "trim",
            Edit::Rotate { .. } => "rotate",
            Edit::Text { .. } => "text",
            Edit::Filter { .. } => "filter",
        }
    }

    pub fn parameters(&self) -> Value {
        match self {
            Edit::Crop {
                x,
                y,
                width,
                height,
            } => json!({ "x": x, "y": y, "width": width, "height": height }),
            Edit::Trim { start_ms, end_ms } => json!({ "start_ms": start_ms, "end_ms": end_ms }),
            Edit::Rotate { degrees } => json!({ "degrees": degrees }),
            Edit::Text { body } => json!({ "body": body }),
            Edit::Filter { name } => json!({ "name": name }),
        }
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        match self {
            Edit::Crop { width, height, .. } if *width == 0 || *height == 0 => Err(CoreError::msg(
                "crop width and height must be greater than zero",
            )),
            Edit::Trim { start_ms, end_ms } if start_ms >= end_ms => {
                Err(CoreError::msg("trim end must be after the start"))
            }
            Edit::Text { body } if body.trim().is_empty() => Err(CoreError::msg("text is empty")),
            Edit::Filter { name } if name.trim().is_empty() => {
                Err(CoreError::msg("filter name is empty"))
            }
            Edit::Filter { name } if is_generative(name) => {
                Err(CoreError::msg("generative effects are not part of v1"))
            }
            _ => Ok(()),
        }
    }
}

fn is_generative(name: &str) -> bool {
    let n = name.to_ascii_lowercase().replace('_', "-");
    n.contains("generative") || n.contains("face-swap") || n.contains("object-removal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_action_name() {
        let edit = Edit::Crop {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        };
        assert_eq!(edit.action(), "c2pa.cropped");
        assert!(edit.validate().is_ok());
    }

    #[test]
    fn generative_filter_is_rejected() {
        let edit = Edit::Filter {
            name: "generative-fill".into(),
        };
        assert!(edit.validate().is_err());
    }
}
