import Foundation
import Security
import XCTest

@testable import DeltaSyncCore

/// Environment (set by `scripts/test.sh`):
///   DELTA_SYNC_TEST_IDENTITY  JSON from `cargo run --example test_identity`
///   DELTA_SYNC_TEST_FIXTURES  crates/core/tests/fixtures
///
/// The key here is a software P-256 key: the simulator has no Secure Enclave.
/// What these tests cover is the Swift to Rust bridge, the DeviceSigner
/// callback, DER to r||s, and verification of the signed JPEG, MOV and MP4.
/// Secure Enclave key creation and signing need a physical device.
final class DeviceSignerTests: XCTestCase {
    private struct Identity: Decodable {
        let ca_pem: String
        let chain_pem: String
        let x963_private_base64: String
    }

    private func identity() throws -> (Identity, SecKey) {
        guard let path = ProcessInfo.processInfo.environment["DELTA_SYNC_TEST_IDENTITY"] else {
            throw XCTSkip("DELTA_SYNC_TEST_IDENTITY is not set; run scripts/test.sh")
        }
        let identity = try JSONDecoder().decode(
            Identity.self, from: Data(contentsOf: URL(fileURLWithPath: path)))
        let raw = Data(base64Encoded: identity.x963_private_base64)!
        var error: Unmanaged<CFError>?
        guard let key = SecKeyCreateWithData(
            raw as CFData,
            [
                kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
                kSecAttrKeyClass: kSecAttrKeyClassPrivate,
            ] as CFDictionary,
            &error
        ) else {
            throw error!.takeRetainedValue()
        }
        return (identity, key)
    }

    private func fixture(_ name: String) throws -> URL {
        guard let dir = ProcessInfo.processInfo.environment["DELTA_SYNC_TEST_FIXTURES"] else {
            throw XCTSkip("DELTA_SYNC_TEST_FIXTURES is not set; run scripts/test.sh")
        }
        return URL(fileURLWithPath: dir).appendingPathComponent(name)
    }

    private func verdict(_ path: String, ca: String) throws -> [String: Any] {
        let json = verifyFileJson(path: path, caPem: ca, selfDeclaredAi: false)
        return try JSONSerialization.jsonObject(with: Data(json.utf8)) as! [String: Any]
    }

    func testSignsJpegMovAndMp4ThroughTheForeignSigner() throws {
        let (identity, key) = try identity()
        let signer = SecureEnclaveDeviceSigner(
            privateKey: key, certificateChainPEM: identity.chain_pem)
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("delta-sync-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }

        for name in ["sample.jpg", "sample.mov", "sample.mp4"] {
            let out = dir.appendingPathComponent(name)
            try signCaptureFile(
                inPath: try fixture(name).path, outPath: out.path, signer: signer)
            let result = try verdict(out.path, ca: identity.ca_pem)
            XCTAssertEqual(result["label"] as? String, "HM", "\(name): \(result)")
            XCTAssertEqual(result["trusted"] as? Bool, true, "\(name): \(result)")
        }
    }

    func testSigningWithoutACertificateFailsClearly() throws {
        let (identity, key) = try identity()
        _ = identity
        let signer = SecureEnclaveDeviceSigner(privateKey: key)
        let out = FileManager.default.temporaryDirectory
            .appendingPathComponent("delta-sync-\(UUID().uuidString).jpg")
        defer { try? FileManager.default.removeItem(at: out) }
        XCTAssertThrowsError(
            try signCaptureFile(
                inPath: try fixture("sample.jpg").path, outPath: out.path, signer: signer)
        ) { error in
            XCTAssertTrue("\(error)".contains("enrol"), "\(error)")
        }
        XCTAssertFalse(FileManager.default.fileExists(atPath: out.path))
    }

    func testSoftwareKeyProducesRawSignatureAndCsr() throws {
        let (identity, key) = try identity()
        let signer = SecureEnclaveDeviceSigner(
            privateKey: key, certificateChainPEM: identity.chain_pem)
        let signature = try signer.sign(data: Data("hello".utf8))
        XCTAssertEqual(signature.count, 64)
        XCTAssertEqual(try signer.publicKeyPoint().count, 65)
        XCTAssertTrue(try signer.makeCsr(commonName: "test").contains("BEGIN CERTIFICATE REQUEST"))
    }

    func testDerToRawPadsAndStripsIntegers() throws {
        // r has a leading zero (high bit set), s is short.
        let r = [UInt8](repeating: 0xFF, count: 32)
        let s: [UInt8] = [0x01, 0x02]
        var der: [UInt8] = [0x02, 33, 0x00] + r + [0x02, UInt8(s.count)] + s
        der = [0x30, UInt8(der.count)] + der
        let raw = try ECDSASignature.derToRaw(Data(der))
        XCTAssertEqual(raw.count, 64)
        XCTAssertEqual([UInt8](raw.prefix(32)), r)
        XCTAssertEqual([UInt8](raw.suffix(32)), [UInt8](repeating: 0, count: 30) + s)
        XCTAssertThrowsError(try ECDSASignature.derToRaw(Data([0x01, 0x02])))
        XCTAssertThrowsError(try ECDSASignature.derToRaw(Data([0x30, 0x03, 0x02, 0x01, 0x01])))
    }
}
