//! Video through the whole chain: a device captures, the service seals a
//! transcode, a verifier labels the result. Plus the cases that must never
//! be HM.

mod common;

use std::fs;
use std::process::Command;
use std::sync::Arc;

use common::mock_tsa::MockTsa;
use common::{fixture, P256Signer, Scratch};
use delta_sync_core::{
    seal_derivative, sign_capture_file, sign_capture_file_with, verify_bytes_with,
    verify_file_with, DevAuthority, Edit, SealRequest, Timestamping, VerifyOptions,
};
use time::{Duration, OffsetDateTime};

const MP4: &str = "video/mp4";
const MOV: &str = "video/quicktime";

fn options(authority: &DevAuthority) -> VerifyOptions {
    VerifyOptions::new(authority.ca_pem())
}

/// Capture `name` with a fresh device and return the signed bytes.
fn capture(authority: &DevAuthority, scratch: &Scratch, name: &str) -> Vec<u8> {
    let out = scratch.path(&format!("captured-{name}"));
    sign_capture_file(&fixture(name), &out, P256Signer::device(authority)).unwrap();
    fs::read(out).unwrap()
}

#[allow(clippy::too_many_arguments)]
fn seal(
    authority: &DevAuthority,
    original: &[u8],
    original_mime: &str,
    rendered: &[u8],
    rendered_mime: &str,
    edits: &[Edit],
    options: &VerifyOptions,
) -> Result<Vec<u8>, delta_sync_core::CoreError> {
    seal_derivative(
        &SealRequest {
            original,
            original_mime,
            rendered,
            rendered_mime,
            declared_ai: false,
            edits,
            resized: true,
        },
        P256Signer::service(authority),
        &Timestamping::None,
        options,
    )
}

fn transcoded() -> Vec<u8> {
    fs::read(fixture("sample-transcoded.mp4")).unwrap()
}

/// Read the manifests without judging trust: these tests look at structure.
fn reader_for(mime: &str, bytes: &[u8]) -> c2pa::Reader {
    let context = c2pa::Context::new()
        .with_settings(r#"{"verify": {"verify_trust": false}}"#)
        .unwrap();
    c2pa::Reader::from_context(context)
        .with_stream(mime, std::io::Cursor::new(bytes))
        .unwrap()
}

/// True when the asset's active manifest hashes its media with
/// `c2pa.hash.bmff.v3` and a Merkle map.
fn has_merkle_bmff_hash(bytes: &[u8], mime: &str, options: &VerifyOptions) -> bool {
    let _ = options;
    let reader = reader_for(mime, bytes);
    let json: serde_json::Value = serde_json::from_str(&reader.json()).unwrap();
    let active = json["active_manifest"].as_str().unwrap().to_string();
    let assertions = json["manifests"][&active]["assertions"].as_array().unwrap();
    assertions.iter().any(|a| {
        a["label"] == "c2pa.hash.bmff.v3"
            && a["data"]["merkle"]
                .as_array()
                .is_some_and(|merkle| !merkle.is_empty())
    })
}

#[test]
fn a_device_capture_of_a_mov_and_an_mp4_uses_bmff_v3_with_merkle_chunks() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("merkle");
    for (name, mime) in [("sample.mov", MOV), ("sample.mp4", MP4)] {
        let bytes = capture(&authority, &scratch, name);
        assert!(
            has_merkle_bmff_hash(&bytes, mime, &options(&authority)),
            "{name}"
        );
        let verdict = verify_bytes_with(&bytes, mime, &options(&authority), false);
        assert_eq!(verdict.label, "HM", "{name}: {verdict:?}");
        assert_eq!(verdict.signers, vec!["device"]);
    }
}

#[test]
fn capture_then_seal_a_transcode_then_verify_is_hm() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("roundtrip");
    // The MOV an iPhone records, transcoded to the MP4 the service delivers.
    for (name, original_mime) in [("sample.mov", MOV), ("sample.mp4", MP4)] {
        let captured = capture(&authority, &scratch, name);
        let sealed = seal(
            &authority,
            &captured,
            original_mime,
            &transcoded(),
            MP4,
            &[],
            &options(&authority),
        )
        .unwrap();
        assert!(has_merkle_bmff_hash(&sealed, MP4, &options(&authority)));
        let verdict = verify_bytes_with(&sealed, MP4, &options(&authority), false);
        assert_eq!(verdict.label, "HM", "{name}: {verdict:?}");
        assert!(verdict.trusted, "{name}: {verdict:?}");
        assert_eq!(verdict.signers, vec!["service", "device"], "{name}");
        assert!(verdict.actions.contains(&"c2pa.transcoded".into()));
        assert!(verdict.actions.contains(&"c2pa.created".into()));

        // The same bytes through the file API.
        let file = scratch.path(&format!("sealed-{name}.mp4"));
        fs::write(&file, &sealed).unwrap();
        let verdict = verify_file_with(&file, None, &options(&authority), false);
        assert_eq!(verdict.label, "HM", "{verdict:?}");
    }
}

#[test]
fn one_changed_byte_in_a_sealed_video_is_unknown() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("tamper");
    let captured = capture(&authority, &scratch, "sample.mp4");
    let sealed = seal(
        &authority,
        &captured,
        MP4,
        &transcoded(),
        MP4,
        &[],
        &options(&authority),
    )
    .unwrap();
    assert_eq!(
        verify_bytes_with(&sealed, MP4, &options(&authority), false).label,
        "HM"
    );
    // The manifest is at the front (faststart): a byte a quarter of the
    // payload from the end is media the hash covers.
    let mut tampered = sealed.clone();
    let at = tampered.len() - transcoded().len() / 4;
    tampered[at] ^= 0xff;
    let verdict = verify_bytes_with(&tampered, MP4, &options(&authority), false);
    assert_eq!(verdict.label, "Unknown", "{verdict:?}");

    // And the original capture inside the seal: change it before sealing.
    let mut bad_origin = captured.clone();
    let at = bad_origin.len() - fs::read(fixture("sample.mp4")).unwrap().len() / 4;
    bad_origin[at] ^= 0xff;
    let resealed = seal(
        &authority,
        &bad_origin,
        MP4,
        &transcoded(),
        MP4,
        &[],
        &options(&authority),
    );
    // Either the seal refuses the damaged origin or the label drops.
    if let Ok(resealed) = resealed {
        let verdict = verify_bytes_with(&resealed, MP4, &options(&authority), false);
        assert_eq!(verdict.label, "Unknown", "{verdict:?}");
    }
}

#[test]
fn mute_is_a_c2pa_edited_audio_removed_and_stays_hm() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("mute");
    let captured = capture(&authority, &scratch, "sample.mp4");
    let sealed = seal(
        &authority,
        &captured,
        MP4,
        &transcoded(),
        MP4,
        &[Edit::Mute],
        &options(&authority),
    )
    .unwrap();
    let verdict = verify_bytes_with(&sealed, MP4, &options(&authority), false);
    assert_eq!(verdict.label, "HM", "{verdict:?}");
    assert!(verdict.summary.contains("mute"), "{verdict:?}");
    assert!(verdict.actions.contains(&"c2pa.edited".into()));

    let reader = reader_for(MP4, &sealed);
    let json = reader.json();
    assert!(json.contains("audio removed"), "{json}");
}

#[test]
fn the_poster_is_sealed_as_its_own_derivative_of_the_video() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("poster");
    let captured = capture(&authority, &scratch, "sample.mp4");
    let poster = fs::read(fixture("sample.jpg")).unwrap();
    let sealed = seal(
        &authority,
        &captured,
        MP4,
        &poster,
        "image/jpeg",
        &[Edit::Cover { at_ms: 500 }],
        &options(&authority),
    )
    .unwrap();
    let verdict = verify_bytes_with(&sealed, "image/jpeg", &options(&authority), false);
    assert_eq!(verdict.label, "HM", "{verdict:?}");
    assert!(verdict.summary.contains("cover"), "{verdict:?}");
    assert_eq!(verdict.signers, vec!["service", "device"]);

    // The poster's parent is the video capture.
    let reader = reader_for("image/jpeg", &sealed);
    let json: serde_json::Value = serde_json::from_str(&reader.json()).unwrap();
    let active = json["active_manifest"].as_str().unwrap();
    assert_eq!(
        json["manifests"][active]["ingredients"][0]["relationship"],
        "parentOf"
    );

    // A poster that is not under a seal is only a JPEG.
    let verdict = verify_bytes_with(&poster, "image/jpeg", &options(&authority), false);
    assert_eq!(verdict.label, "Unknown");
}

/// How long the short-lived certificates of the expiry tests live. c2pa will
/// not sign with a certificate that has expired, so the capture is signed while
/// it is valid and the tests wait for it to lapse.
const SHORT_LIFE: std::time::Duration = std::time::Duration::from_secs(6);

fn wait_until_expired(not_after: OffsetDateTime) {
    let remaining = (not_after - OffsetDateTime::now_utc()).max(Duration::ZERO);
    std::thread::sleep(std::time::Duration::from_millis(
        remaining.whole_milliseconds() as u64 + 1500,
    ));
}

#[test]
fn an_expired_device_certificate_with_a_trusted_time_stamp_still_verifies() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("expired");
    let now = OffsetDateTime::now_utc();
    let not_after = now + SHORT_LIFE;
    let signer = P256Signer::device_between(&authority, now - Duration::minutes(5), not_after);
    // The TSA stamps the moment of signing, inside the certificate's life.
    let tsa = Arc::new(MockTsa::new(now));
    let stamped = scratch.path("stamped.mp4");
    sign_capture_file_with(
        &fixture("sample.mp4"),
        &stamped,
        signer.clone(),
        &Timestamping::Provider(tsa.clone()),
    )
    .unwrap();
    let plain = scratch.path("plain.mp4");
    sign_capture_file(&fixture("sample.mp4"), &plain, signer).unwrap();
    let trusting = options(&authority).with_tsa_anchors(tsa.root_pem.clone());

    // While the certificate is valid both are HM.
    let captured = fs::read(&stamped).unwrap();
    assert_eq!(
        verify_bytes_with(&captured, MP4, &options(&authority), false).label,
        "HM"
    );

    wait_until_expired(not_after);

    // Without the TSA's root there is nothing to vouch for the time: the
    // certificate is simply expired.
    let verdict = verify_bytes_with(&captured, MP4, &options(&authority), false);
    assert_eq!(verdict.label, "Unknown", "{verdict:?}");

    // With it, the signature is dated inside the certificate's life.
    let verdict = verify_bytes_with(&captured, MP4, &trusting, false);
    assert_eq!(verdict.label, "HM", "{verdict:?}");
    assert!(verdict.trusted, "{verdict:?}");

    // The service can seal it, and the seal verifies too.
    let sealed = seal(
        &authority,
        &captured,
        MP4,
        &transcoded(),
        MP4,
        &[],
        &trusting,
    )
    .unwrap();
    let verdict = verify_bytes_with(&sealed, MP4, &trusting, false);
    assert_eq!(verdict.label, "HM", "{verdict:?}");

    // The same capture without a time stamp is Unknown even to a verifier
    // that trusts the TSA.
    let verdict = verify_bytes_with(&fs::read(&plain).unwrap(), MP4, &trusting, false);
    assert_eq!(verdict.label, "Unknown", "{verdict:?}");
}

#[test]
fn a_time_stamp_dated_after_the_certificate_expired_does_not_rescue_it() {
    let authority = DevAuthority::generate().unwrap();
    let scratch = Scratch::new("late");
    let now = OffsetDateTime::now_utc();
    let not_after = now + SHORT_LIFE;
    let signer = P256Signer::device_between(&authority, now - Duration::minutes(5), not_after);
    // A TSA that says the signature was made after the certificate lapsed.
    let tsa = Arc::new(MockTsa::new(not_after + Duration::hours(1)));
    let out = scratch.path("late.mp4");
    let signed = sign_capture_file_with(
        &fixture("sample.mp4"),
        &out,
        signer,
        &Timestamping::Provider(tsa.clone()),
    );
    // c2pa checks what it just signed and refuses a signature that its own time
    // stamp puts outside the certificate's life. If a file like that ever
    // existed it would be Unknown to the verifier, which this part checks.
    match signed {
        Err(err) => assert!(err.to_string().contains("expired"), "{err}"),
        Ok(()) => {
            wait_until_expired(not_after);
            let trusting = options(&authority).with_tsa_anchors(tsa.root_pem.clone());
            let verdict = verify_bytes_with(&fs::read(&out).unwrap(), MP4, &trusting, false);
            assert_eq!(verdict.label, "Unknown", "{verdict:?}");
        }
    }
}

/// A clip a few megabytes long, so the Merkle tree has more than one chunk.
fn long_clip(scratch: &Scratch) -> Option<std::path::PathBuf> {
    let out = scratch.path("long.mp4");
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("testsrc2=size=320x240:rate=25,noise=alls=60:allf=t")
        .args([
            "-t", "6", "-c:v", "libx264", "-crf", "8", "-pix_fmt", "yuv420p",
        ])
        .args(["-movflags", "+faststart"])
        .arg(&out)
        .status()
        .ok()?;
    (status.success() && fs::metadata(&out).ok()?.len() > 2 * 1024 * 1024).then_some(out)
}

#[test]
fn a_byte_flipped_in_the_second_merkle_chunk_is_caught() {
    let scratch = Scratch::new("long");
    let Some(clip) = long_clip(&scratch) else {
        eprintln!("skipped: ffmpeg is not installed or could not make a clip over 2 MB");
        return;
    };
    let authority = DevAuthority::generate().unwrap();
    let signed = scratch.path("long-signed.mp4");
    sign_capture_file(&clip, &signed, P256Signer::device(&authority)).unwrap();
    let verdict = verify_file_with(&signed, None, &options(&authority), false);
    assert_eq!(verdict.label, "HM", "{verdict:?}");

    let mut bytes = fs::read(&signed).unwrap();
    let at = bytes.len() - 1024 * 1024 - 17; // past the first 1024 KB chunk
    bytes[at] ^= 0x01;
    let tampered = scratch.path("long-tampered.mp4");
    fs::write(&tampered, bytes).unwrap();
    let verdict = verify_file_with(&tampered, None, &options(&authority), false);
    assert_eq!(verdict.label, "Unknown", "{verdict:?}");
}
