//! Video end to end through the HTTP API: a device captures a MOV or MP4, the
//! media processor asks `/internal/seal` for the transcode, `/v1/verify`
//! labels the result. Keys and certificates are generated at run time and
//! nothing uses the network.

use std::sync::Arc;

use axum::http::StatusCode;
use serde_json::Value;

mod common;
use common::*;

const TOKEN: &str = "private-test-media-service-token-123456";
const MP4: &str = "video/mp4";
const MOV: &str = "video/quicktime";

struct Scenario {
    router: axum::Router,
}

impl Scenario {
    fn new() -> Self {
        let (router, _) = test_app_with(|config| config.seal_token = Some(TOKEN.to_string()));
        Self { router }
    }

    /// Capture `fixture` on a freshly enrolled device.
    async fn capture(&self, fixture: &str) -> Vec<u8> {
        let (device, chain, _) = enrolled_device(&self.router).await;
        let out = tempfile_dir().join(format!("captured-{fixture}"));
        delta_sync_core::sign_capture_file(
            &core_fixture(fixture),
            &out,
            Arc::new(EnrolledSigner {
                key: device.signing_key,
                chain,
            }),
        )
        .unwrap();
        std::fs::read(out).unwrap()
    }

    async fn seal(
        &self,
        original: (&[u8], &str),
        rendered: (&[u8], &str),
        edits: Option<&str>,
    ) -> (StatusCode, Vec<u8>) {
        let mut parts: Vec<(&str, Option<&str>, &[u8])> = vec![
            ("original", Some(original.1), original.0),
            ("rendered", Some(rendered.1), rendered.0),
        ];
        if let Some(edits) = edits {
            parts.push(("edits", None, edits.as_bytes()));
        }
        raw_request(
            &self.router,
            "/internal/seal",
            &[
                ("content-type", "multipart/form-data; boundary=sealboundary"),
                ("authorization", &format!("Bearer {TOKEN}")),
            ],
            multipart("sealboundary", &parts),
        )
        .await
    }

    async fn verify(&self, bytes: &[u8], mime: &str) -> Value {
        let (status, body) = raw_request(
            &self.router,
            "/v1/verify",
            &[("x-mime-type", mime)],
            bytes.to_vec(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        serde_json::from_slice(&body).unwrap()
    }
}

fn transcoded() -> Vec<u8> {
    std::fs::read(core_fixture("sample-transcoded.mp4")).unwrap()
}

#[tokio::test]
async fn capture_seal_verify_round_trip_is_hm_for_mov_and_mp4() {
    let scenario = Scenario::new();
    for (fixture, mime) in [("sample.mov", MOV), ("sample.mp4", MP4)] {
        let captured = scenario.capture(fixture).await;
        let verdict = scenario.verify(&captured, mime).await;
        assert_eq!(verdict["label"], "HM", "{fixture}: {verdict}");
        assert_eq!(verdict["signers"], serde_json::json!(["device"]));

        let (status, sealed) = scenario
            .seal((&captured, mime), (&transcoded(), MP4), None)
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{fixture}: {}",
            String::from_utf8_lossy(&sealed)
        );
        let verdict = scenario.verify(&sealed, MP4).await;
        assert_eq!(verdict["label"], "HM", "{fixture}: {verdict}");
        assert_eq!(verdict["trusted"], true, "{fixture}: {verdict}");
        assert_eq!(
            verdict["signers"],
            serde_json::json!(["service", "device"]),
            "{fixture}"
        );
        let actions = verdict["actions"].to_string();
        assert!(actions.contains("c2pa.transcoded"), "{actions}");
    }
}

#[tokio::test]
async fn one_changed_byte_in_a_sealed_video_is_unknown() {
    let scenario = Scenario::new();
    let captured = scenario.capture("sample.mp4").await;
    let (status, mut sealed) = scenario
        .seal((&captured, MP4), (&transcoded(), MP4), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let at = sealed.len() - transcoded().len() / 4;
    sealed[at] ^= 0xff;
    let verdict = scenario.verify(&sealed, MP4).await;
    assert_eq!(verdict["label"], "Unknown", "{verdict}");
}

#[tokio::test]
async fn a_tampered_capture_is_not_sealed() {
    let scenario = Scenario::new();
    let mut captured = scenario.capture("sample.mp4").await;
    let original = std::fs::read(core_fixture("sample.mp4")).unwrap();
    let at = captured.len() - original.len() / 4;
    captured[at] ^= 0xff;
    let (status, _) = scenario
        .seal((&captured, MP4), (&transcoded(), MP4), None)
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn mute_and_cover_are_accepted_edits_and_keep_hm() {
    let scenario = Scenario::new();
    let captured = scenario.capture("sample.mp4").await;

    // Mute the video: c2pa.edited, "audio removed".
    let (status, muted) = scenario
        .seal(
            (&captured, MP4),
            (&transcoded(), MP4),
            Some(r#"[{"op":"mute"}]"#),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&muted)
    );
    let verdict = scenario.verify(&muted, MP4).await;
    assert_eq!(verdict["label"], "HM", "{verdict}");
    assert!(
        verdict["summary"].as_str().unwrap().contains("mute"),
        "{verdict}"
    );

    // The poster is its own derivative: the video is its parent.
    let poster = std::fs::read(core_fixture("sample.jpg")).unwrap();
    let (status, sealed_poster) = scenario
        .seal(
            (&captured, MP4),
            (&poster, "image/jpeg"),
            Some(r#"[{"op":"cover","at_ms":500}]"#),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&sealed_poster)
    );
    let verdict = scenario.verify(&sealed_poster, "image/jpeg").await;
    assert_eq!(verdict["label"], "HM", "{verdict}");
    assert!(
        verdict["summary"].as_str().unwrap().contains("cover"),
        "{verdict}"
    );
    assert_eq!(verdict["signers"], serde_json::json!(["service", "device"]));

    // An unsealed poster is just a JPEG.
    let verdict = scenario.verify(&poster, "image/jpeg").await;
    assert_eq!(verdict["label"], "Unknown", "{verdict}");
}

#[tokio::test]
async fn a_quicktime_file_is_recognised_without_a_mime_header() {
    let scenario = Scenario::new();
    let captured = scenario.capture("sample.mov").await;
    let (status, body) = raw_request(&scenario.router, "/v1/verify", &[], captured).await;
    assert_eq!(status, StatusCode::OK);
    let verdict: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(verdict["label"], "HM", "{verdict}");
}
