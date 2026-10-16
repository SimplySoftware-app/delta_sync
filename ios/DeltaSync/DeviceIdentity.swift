import DeltaSyncCore
import Foundation
import Security

enum IdentityError: Error, LocalizedError {
    case secureEnclave(String)
    case softwareKey(OSStatus)

    var errorDescription: String? {
        switch self {
        case .secureEnclave(let message):
            return message
        case .softwareKey(let status):
            return "The development software key could not be created (OSStatus \(status))."
        }
    }
}

enum KeyBacking: String {
    case secureEnclave = "Secure Enclave"
    case simulatorSoftware = "Simulator software key"
}

struct DeviceIdentity {
    static let enclaveTag = "cloud.deltasync.capture.signing"
    static let softwareTag = "cloud.deltasync.capture.signing.software"

    let backing: KeyBacking
    let privateKey: SecKey

    static func loadOrCreate(allowSoftwareFallback: Bool) throws -> DeviceIdentity {
        do {
            let key = try SecureEnclaveDeviceSigner.loadOrCreateKey(tag: enclaveTag)
            return DeviceIdentity(backing: .secureEnclave, privateKey: key)
        } catch {
            guard allowSoftwareFallback else {
                throw IdentityError.secureEnclave(
                    "Secure Enclave is unavailable. Use a physical iPhone. The simulator cannot create this key. (\(error.localizedDescription))"
                )
            }
            let key = try softwareKey()
            return DeviceIdentity(backing: .simulatorSoftware, privateKey: key)
        }
    }

    private static func copyKey(tag: String, secureEnclave: Bool) -> SecKey? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecAttrApplicationTag as String: tag,
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecReturnRef as String: true
        ]
        if secureEnclave {
            query[kSecAttrTokenID as String] = kSecAttrTokenIDSecureEnclave
        }
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess else { return nil }
        return item as! SecKey?
    }

    private static func softwareKey() throws -> SecKey {
        if let existing = copyKey(tag: softwareTag, secureEnclave: false) {
            return existing
        }
        let attributes: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrKeySizeInBits as String: 256,
            kSecPrivateKeyAttrs as String: [
                kSecAttrIsPermanent as String: true,
                kSecAttrApplicationTag as String: softwareTag
            ]
        ]
        var error: Unmanaged<CFError>?
        guard let key = SecKeyCreateRandomKey(attributes as CFDictionary, &error) else {
            let code = (error?.takeRetainedValue() as Error? as NSError?)?.code
            let status = code.map { OSStatus(truncatingIfNeeded: $0) } ?? errSecParam
            throw IdentityError.softwareKey(status)
        }
        return key
    }
}
