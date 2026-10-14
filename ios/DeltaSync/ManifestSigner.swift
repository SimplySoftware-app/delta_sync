import C2PA
import Foundation
import Security

enum ManifestError: Error, LocalizedError {
    case signing(String)

    var errorDescription: String? {
        switch self {
        case .signing(let message):
            return message
        }
    }
}

enum ManifestSigner {
    static func signJpeg(
        _ jpeg: Data,
        certificateChainPEM: String,
        identity: DeviceIdentity
    ) throws -> Data {
        let manifest = """
        {
          "claim_generator": "DeltaSync iOS/0.1.0",
          "title": "DeltaSync capture",
          "format": "image/jpeg",
          "assertions": [
            {
              "label": "c2pa.actions",
              "data": {
                "actions": [
                  {
                    "action": "c2pa.created",
                    "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
                  }
                ]
              }
            }
          ]
        }
        """
        let signer: Signer
        switch identity.backing {
        case .secureEnclave:
            let config = SecureEnclaveSignerConfig(keyTag: DeviceIdentity.enclaveTag)
            signer = try Signer(
                algorithm: .es256,
                certificateChainPEM: certificateChainPEM,
                tsa: nil,
                secureEnclaveConfig: config
            )
        case .simulatorSoftware:
            let privateKey = identity.privateKey
            signer = try Signer(
                algorithm: .es256,
                certificateChainPEM: certificateChainPEM,
                tsa: nil
            ) { data in
                try rawSignature(privateKey: privateKey, data: data)
            }
        }
        let builder = try Builder(manifestJSON: manifest)
        let source = try Stream(data: jpeg)
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("deltasync-\(UUID().uuidString).jpg")
        let destination = try Stream(writeTo: url)
        do {
            try builder.sign(
                format: "image/jpeg",
                source: source,
                destination: destination,
                signer: signer
            )
            return try Data(contentsOf: url)
        } catch {
            throw ManifestError.signing(error.localizedDescription)
        }
    }

    /// COSE ES256 wants the raw P1363 signature, r || s.
    private static func rawSignature(privateKey: SecKey, data: Data) throws -> Data {
        var error: Unmanaged<CFError>?
        guard let der = SecKeyCreateSignature(
            privateKey,
            .ecdsaSignatureMessageX962SHA256,
            data as CFData,
            &error
        ) as Data? else {
            throw ManifestError.signing("Software key signing failed.")
        }
        return try derToP1363(der)
    }

    private static func derToP1363(_ der: Data) throws -> Data {
        let bytes = [UInt8](der)
        guard bytes.count > 6, bytes[0] == 0x30 else {
            throw ManifestError.signing("ECDSA signature was not DER.")
        }
        var index = 2
        func integer() throws -> Data {
            guard bytes[index] == 0x02 else {
                throw ManifestError.signing("ECDSA signature was not DER.")
            }
            index += 1
            let length = Int(bytes[index])
            index += 1
            let value = Data(bytes[index..<(index + length)])
            index += length
            return value
        }
        func component(_ value: Data) -> Data {
            let trimmed = value.first == 0 ? value.dropFirst() : value[...]
            return Data(repeating: 0, count: max(0, 32 - trimmed.count)) + trimmed.suffix(32)
        }
        let raw = component(try integer()) + component(try integer())
        guard raw.count == 64 else {
            throw ManifestError.signing("ECDSA signature was not 64 bytes.")
        }
        return raw
    }
}
