//! Delta Sync core.
//!
//! Host apps capture and render. This crate signs a C2PA manifest at capture
//! and a new manifest for each edit, then reads that chain back into an
//! HM / AI / Unknown label.
//!
//! UniFFI (`ffi` feature) is the native surface. WASM builds use
//! `--no-default-features` and only export the label rules and edit
//! descriptions. Byte-level signing stays on the native `c2pa` feature
//! because that crate is not part of the wasm build.

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!();

mod edit;
mod error;
mod keys;
mod label;

#[cfg(feature = "c2pa")]
mod sign;

#[cfg(all(feature = "ffi", feature = "c2pa"))]
mod ffi_api;

#[cfg(all(target_arch = "wasm32", not(feature = "ffi")))]
mod wasm_api;

pub use edit::Edit;
pub use error::CoreError;
pub use keys::{generate_device_key, DevAuthority, DeviceKey, IssuedCert};
pub use label::{assign_label, CredentialReport, Label, LabelVerdict};

#[cfg(feature = "c2pa")]
pub use sign::{
    sign_capture, sign_derivative, sign_derivative_with_edits, sign_edit, verify_bytes,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DIGITAL_CAPTURE: &str = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture";
pub const TRAINED_ALGORITHMIC: &str =
    "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia";
pub const COMPOSITE_TRAINED: &str =
    "http://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia";

/// Claim-generator name written into manifests. HM requires this name.
pub const GENERATOR_NAME: &str = "Delta Sync";
