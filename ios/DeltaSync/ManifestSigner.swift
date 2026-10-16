import DeltaSyncCore
import Foundation

enum ManifestError: Error, LocalizedError {
    case signing(String)

    var errorDescription: String? {
        switch self {
        case .signing(let message):
            return message
        }
    }
}

/// Signs a capture with the Delta Sync core. The Rust core builds the C2PA
/// manifest; the device key signs through `SecureEnclaveDeviceSigner`.
enum ManifestSigner {
    static func signJpeg(_ jpeg: Data, signer: SecureEnclaveDeviceSigner) throws -> Data {
        let directory = FileManager.default.temporaryDirectory
        let id = UUID().uuidString
        let input = directory.appendingPathComponent("deltasync-\(id)-in.jpg")
        let output = directory.appendingPathComponent("deltasync-\(id)-out.jpg")
        defer {
            try? FileManager.default.removeItem(at: input)
            try? FileManager.default.removeItem(at: output)
        }
        do {
            try jpeg.write(to: input)
            try signCaptureFile(inPath: input.path, outPath: output.path, signer: signer)
            return try Data(contentsOf: output)
        } catch {
            throw ManifestError.signing(error.localizedDescription)
        }
    }
}
