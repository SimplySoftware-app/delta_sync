use std::env;
use std::fs;

use delta_sync_core::{generate_device_key, sign_capture, sign_edit, DevAuthority, Edit};

fn main() {
    let authority = DevAuthority::generate().expect("dev CA");
    let device = generate_device_key().expect("device key");
    let issued = authority.issue(&device.public_pem, "sample").expect("cert");
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 32, 0, 0, 0, 24, 8,
        2, 0, 0, 0, 20, 49, 104, 99, 0, 0, 0, 36, 73, 68, 65, 84, 120, 156, 99, 120, 180, 49, 138,
        166, 136, 97, 212, 130, 81, 11, 70, 45, 24, 181, 96, 212, 130, 81, 11, 70, 45, 24, 181, 96,
        104, 88, 0, 0, 255, 65, 199, 76, 245, 107, 255, 18, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
        96, 130,
    ];
    let captured = sign_capture(
        &png,
        "image/png",
        &device.private_pem,
        &issued.certificate_pem,
        &issued.ca_pem,
    )
    .expect("capture");
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
    .expect("crop");
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/delta-sync-sample.png".into());
    fs::write(&path, cropped).expect("write");
    println!("{path}");
}
