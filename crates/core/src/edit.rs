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
    /// The audio track was removed. Recorded as `c2pa.edited` with the
    /// description [`AUDIO_REMOVED`], which the label rules know.
    Mute,
    /// A frame of the video was picked as its poster. Recorded as
    /// `c2pa.edited` with the description [`COVER_SELECTED`]. The poster image
    /// is sealed as its own derivative of the video.
    Cover {
        at_ms: u64,
    },
}

/// Description of the `c2pa.edited` action that records a mute.
pub const AUDIO_REMOVED: &str = "audio removed";
/// Description of the `c2pa.edited` action that records a cover frame.
pub const COVER_SELECTED: &str = "cover frame selected";

impl Edit {
    pub fn action(&self) -> &'static str {
        match self {
            Edit::Crop { .. } => "c2pa.cropped",
            Edit::Trim { .. } => "c2pa.trimmed",
            Edit::Rotate { .. } => "c2pa.orientation",
            Edit::Text { .. } => "c2pa.drawing",
            Edit::Filter { .. } => "c2pa.filtered",
            Edit::Mute | Edit::Cover { .. } => "c2pa.edited",
        }
    }

    /// The `description` of the action, for the actions that need one to be
    /// understood (`c2pa.edited` is too generic to be permitted on its own).
    pub fn description(&self) -> Option<&'static str> {
        match self {
            Edit::Mute => Some(AUDIO_REMOVED),
            Edit::Cover { .. } => Some(COVER_SELECTED),
            _ => None,
        }
    }

    /// The action as it is written in the manifest.
    pub fn action_json(&self) -> Value {
        let mut action = json!({ "action": self.action(), "parameters": self.parameters() });
        if let Some(description) = self.description() {
            action["description"] = json!(description);
        }
        action
    }

    pub fn operation(&self) -> &'static str {
        match self {
            Edit::Crop { .. } => "crop",
            Edit::Trim { .. } => "trim",
            Edit::Rotate { .. } => "rotate",
            Edit::Text { .. } => "text",
            Edit::Filter { .. } => "filter",
            Edit::Mute => "mute",
            Edit::Cover { .. } => "cover",
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
            Edit::Mute => json!({}),
            Edit::Cover { at_ms } => json!({ "at_ms": at_ms }),
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
    fn mute_and_cover_are_recorded_as_described_edits() {
        let mute = Edit::Mute;
        assert_eq!(mute.action(), "c2pa.edited");
        assert_eq!(mute.operation(), "mute");
        assert_eq!(mute.action_json()["description"], AUDIO_REMOVED);
        let cover = Edit::Cover { at_ms: 1500 };
        assert_eq!(cover.action(), "c2pa.edited");
        assert_eq!(cover.operation(), "cover");
        assert_eq!(cover.action_json()["description"], COVER_SELECTED);
        assert_eq!(cover.action_json()["parameters"]["at_ms"], 1500);
        let parsed: Edit = serde_json::from_str(r#"{"op":"mute"}"#).unwrap();
        assert_eq!(parsed, Edit::Mute);
        let parsed: Edit = serde_json::from_str(r#"{"op":"cover","at_ms":9}"#).unwrap();
        assert_eq!(parsed, Edit::Cover { at_ms: 9 });
        assert!(Edit::Crop {
            x: 0,
            y: 0,
            width: 1,
            height: 1
        }
        .action_json()
        .get("description")
        .is_none());
    }

    #[test]
    fn generative_filter_is_rejected() {
        let edit = Edit::Filter {
            name: "generative-fill".into(),
        };
        assert!(edit.validate().is_err());
    }
}
