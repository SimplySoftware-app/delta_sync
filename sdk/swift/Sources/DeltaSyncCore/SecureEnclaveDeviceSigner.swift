import Foundation
import Security

/// Errors raised while creating or using the device key.
public enum DeviceKeyError: Error, LocalizedError {
    case secureEnclaveUnavailable(String)
    case keyCreation(OSStatus)
    case publicKey
    case signing(String)

    public var errorDescription: String? {
        switch self {
        case .secureEnclaveUnavailable(let detail):
            return "Secure Enclave is unavailable (use a physical device): \(detail)"
        case .keyCreation(let status):
            return "The device key could not be created (OSStatus \(status))."
        case .publicKey:
            return "The device key did not produce a P-256 public point."
        case .signing(let detail):
            return detail
        }
    }
}

/// A `DeviceSigner` backed by a P-256 key in the Secure Enclave.
///
/// The private key is created inside the Secure Enclave and is never
/// exported. The Rust core builds the C2PA claim and calls `sign(data:)` with
/// the bytes to sign; this class returns the raw `r || s` signature COSE
/// expects (Security.framework returns ASN.1 DER, which is converted here).
///
/// The certificate chain is only known after enrolment: create the signer with
/// the key, enrol (`makeCsr` + the authority's `/v1/device-certificate`), then
/// call `setCertificateChain(_:)` with the PEM the authority returned.
///
/// The simulator has no Secure Enclave. `loadOrCreateKey(tag:)` throws there;
/// `init(privateKey:)` accepts any P-256 `SecKey` so a development build can
/// pass a software key explicitly.
public final class SecureEnclaveDeviceSigner: DeviceSigner, @unchecked Sendable {
    private let privateKey: SecKey
    private let lock = NSLock()
    private var chainPEM: String

    public init(privateKey: SecKey, certificateChainPEM: String = "") {
        self.privateKey = privateKey
        self.chainPEM = certificateChainPEM
    }

    /// Load the Secure Enclave key stored under `tag`, or create it.
    /// The key never leaves the enclave and is only usable on this device
    /// while it is unlocked.
    public static func loadOrCreateKey(tag: String) throws -> SecKey {
        if let existing = copyKey(tag: tag) {
            return existing
        }
        var accessError: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            .privateKeyUsage,
            &accessError
        ) else {
            throw DeviceKeyError.secureEnclaveUnavailable(
                accessError?.takeRetainedValue().localizedDescription ?? "access control"
            )
        }
        let attributes: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrKeySizeInBits as String: 256,
            kSecAttrTokenID as String: kSecAttrTokenIDSecureEnclave,
            kSecPrivateKeyAttrs as String: [
                kSecAttrIsPermanent as String: true,
                kSecAttrApplicationTag as String: Data(tag.utf8),
                kSecAttrAccessControl as String: access,
            ],
        ]
        var error: Unmanaged<CFError>?
        guard let key = SecKeyCreateRandomKey(attributes as CFDictionary, &error) else {
            throw DeviceKeyError.secureEnclaveUnavailable(
                error?.takeRetainedValue().localizedDescription ?? "key generation failed"
            )
        }
        return key
    }

    /// Delete the key stored under `tag`, for example when the user resets enrolment.
    @discardableResult
    public static func deleteKey(tag: String) -> Bool {
        let query: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecAttrApplicationTag as String: Data(tag.utf8),
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
        ]
        let status = SecItemDelete(query as CFDictionary)
        return status == errSecSuccess
    }

    private static func copyKey(tag: String) -> SecKey? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecAttrApplicationTag as String: Data(tag.utf8),
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrTokenID as String: kSecAttrTokenIDSecureEnclave,
            kSecReturnRef as String: true,
        ]
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess else {
            return nil
        }
        return (item as! SecKey)
    }

    /// Install the PEM chain issued by the authority (device certificate first).
    public func setCertificateChain(_ pem: String) {
        lock.lock()
        chainPEM = pem
        lock.unlock()
    }

    /// Uncompressed X9.63 public point (`04 || X || Y`, 65 bytes).
    public func publicKeyPoint() throws -> Data {
        guard let publicKey = SecKeyCopyPublicKey(privateKey) else {
            throw DeviceKeyError.publicKey
        }
        var error: Unmanaged<CFError>?
        guard let raw = SecKeyCopyExternalRepresentation(publicKey, &error) as Data?,
              raw.count == 65, raw.first == 0x04
        else {
            throw DeviceKeyError.publicKey
        }
        return raw
    }

    /// PKCS#10 request for `/v1/device-certificate`, signed by this key.
    public func makeCsr(commonName: String) throws -> String {
        try Csr.make(privateKey: privateKey, commonName: commonName).pem
    }

    // MARK: DeviceSigner

    public func alg() -> SignerAlg { .es256 }

    public func certChain() throws -> String {
        lock.lock()
        defer { lock.unlock() }
        guard chainPEM.contains("BEGIN CERTIFICATE") else {
            throw CoreError.Message(
                reason: "The device has no certificate yet: enrol before signing."
            )
        }
        return chainPEM
    }

    public func sign(data: Data) throws -> Data {
        var error: Unmanaged<CFError>?
        guard let der = SecKeyCreateSignature(
            privateKey,
            .ecdsaSignatureMessageX962SHA256,
            data as CFData,
            &error
        ) as Data? else {
            let detail = error?.takeRetainedValue().localizedDescription ?? "unknown error"
            throw CoreError.Message(reason: "The device key could not sign: \(detail)")
        }
        do {
            return try ECDSASignature.derToRaw(der)
        } catch {
            throw CoreError.Message(reason: error.localizedDescription)
        }
    }
}

/// ASN.1 DER ECDSA signature to the fixed-width `r || s` form COSE uses.
public enum ECDSASignature {
    /// Convert a P-256 DER signature (`SEQUENCE { INTEGER r, INTEGER s }`) to 64 bytes.
    public static func derToRaw(_ der: Data, componentLength: Int = 32) throws -> Data {
        let bytes = [UInt8](der)
        var index = 0
        func readLength() throws -> Int {
            guard index < bytes.count else { throw DeviceKeyError.signing("Truncated ECDSA signature.") }
            let first = Int(bytes[index])
            index += 1
            if first < 0x80 { return first }
            let count = first & 0x7F
            guard count > 0, count <= 2, index + count <= bytes.count else {
                throw DeviceKeyError.signing("Bad ECDSA signature length.")
            }
            var value = 0
            for _ in 0..<count {
                value = (value << 8) | Int(bytes[index])
                index += 1
            }
            return value
        }
        func readInteger() throws -> [UInt8] {
            guard index < bytes.count, bytes[index] == 0x02 else {
                throw DeviceKeyError.signing("ECDSA signature was not DER.")
            }
            index += 1
            let length = try readLength()
            guard length > 0, index + length <= bytes.count else {
                throw DeviceKeyError.signing("Bad ECDSA integer length.")
            }
            var value = Array(bytes[index..<(index + length)])
            index += length
            while value.count > 1, value.first == 0 { value.removeFirst() }
            guard value.count <= componentLength else {
                throw DeviceKeyError.signing("ECDSA integer is too large for P-256.")
            }
            return [UInt8](repeating: 0, count: componentLength - value.count) + value
        }
        guard index < bytes.count, bytes[index] == 0x30 else {
            throw DeviceKeyError.signing("ECDSA signature was not DER.")
        }
        index += 1
        let sequenceLength = try readLength()
        guard index + sequenceLength == bytes.count else {
            throw DeviceKeyError.signing("Bad ECDSA sequence length.")
        }
        let r = try readInteger()
        let s = try readInteger()
        guard index == bytes.count else {
            throw DeviceKeyError.signing("Trailing bytes after ECDSA signature.")
        }
        return Data(r + s)
    }
}
