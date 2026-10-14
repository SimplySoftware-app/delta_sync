import Foundation
import Security

enum CsrError: Error, LocalizedError {
    case publicKey
    case signature(OSStatus)

    var errorDescription: String? {
        switch self {
        case .publicKey:
            return "The device key did not produce a P-256 public point."
        case .signature(let status):
            return "The device key could not sign the certificate request (OSStatus \(status))."
        }
    }
}

enum Csr {
    /// PKCS#10 request signed by a non-exportable P-256 key.
    static func make(privateKey: SecKey, commonName: String) throws -> (pem: String, point: Data) {
        let point = try uncompressedPoint(privateKey)
        let spki = subjectPublicKeyInfo(point)
        let info = certificationRequestInfo(commonName: commonName, spki: spki)
        var error: Unmanaged<CFError>?
        guard let signature = SecKeyCreateSignature(
            privateKey,
            .ecdsaSignatureMessageX962SHA256,
            info as CFData,
            &error
        ) as Data? else {
            throw CsrError.signature((error?.takeRetainedValue() as Error? as NSError?)?.code ?? errSecParam)
        }
        let algorithm = derSequence([
            derOid([0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02])
        ])
        let csr = derSequence([
            info,
            algorithm,
            derBitString(signature)
        ])
        return (pem: pem(csr), point: point)
    }

    static func uncompressedPoint(_ privateKey: SecKey) throws -> Data {
        guard let publicKey = SecKeyCopyPublicKey(privateKey) else {
            throw CsrError.publicKey
        }
        var error: Unmanaged<CFError>?
        guard let raw = SecKeyCopyExternalRepresentation(publicKey, &error) as Data?,
              raw.count == 65, raw.first == 0x04 else {
            throw CsrError.publicKey
        }
        return raw
    }

    private static func subjectPublicKeyInfo(_ point: Data) -> Data {
        let algorithm = derSequence([
            derOid([0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01]),
            derOid([0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07])
        ])
        return derSequence([algorithm, derBitString(point)])
    }

    private static func certificationRequestInfo(commonName: String, spki: Data) -> Data {
        let name = derSequence([
            derSet([
                derSequence([
                    derOid([0x55, 0x04, 0x03]),
                    derUtf8(commonName)
                ])
            ])
        ])
        let attributes = Data([0xA0, 0x00])
        return derSequence([
            derInteger(0),
            name,
            spki,
            attributes
        ])
    }

    private static func pem(_ der: Data) -> String {
        let body = der.base64EncodedString(options: [.lineLength64Characters, .endLineWithLineFeed])
        return "-----BEGIN CERTIFICATE REQUEST-----\n\(body)\n-----END CERTIFICATE REQUEST-----\n"
    }

    private static func derSequence(_ parts: [Data]) -> Data {
        derTag(0x30, Data(parts.joined()))
    }

    private static func derSet(_ parts: [Data]) -> Data {
        derTag(0x31, Data(parts.joined()))
    }

    private static func derInteger(_ value: UInt8) -> Data {
        Data([0x02, 0x01, value])
    }

    private static func derUtf8(_ string: String) -> Data {
        derTag(0x0C, Data(string.utf8))
    }

    private static func derOid(_ body: [UInt8]) -> Data {
        derTag(0x06, Data(body))
    }

    private static func derBitString(_ payload: Data) -> Data {
        derTag(0x03, Data([0x00]) + payload)
    }

    private static func derTag(_ tag: UInt8, _ body: Data) -> Data {
        var out = Data([tag])
        out.append(derLength(body.count))
        out.append(body)
        return out
    }

    private static func derLength(_ length: Int) -> Data {
        if length < 128 {
            return Data([UInt8(length)])
        }
        var value = length
        var bytes: [UInt8] = []
        while value > 0 {
            bytes.insert(UInt8(value & 0xFF), at: 0)
            value >>= 8
        }
        return Data([0x80 | UInt8(bytes.count)] + bytes)
    }
}
