import DeltaSyncCore
import Foundation

@MainActor
final class CaptureModel: ObservableObject {
    @Published var serverURL: String {
        didSet { UserDefaults.standard.set(serverURL, forKey: "serverURL") }
    }
    @Published var appId: String {
        didSet { UserDefaults.standard.set(appId, forKey: "appId") }
    }
    @Published var allowSoftwareKey = false
    @Published var status = "Not prepared."
    @Published var busy = false
    @Published var signedPhoto: Data?
    @Published var backing = ""

    private var identity: DeviceIdentity?
    private var signer: SecureEnclaveDeviceSigner?
    let camera = CameraSession()

    init() {
        serverURL = UserDefaults.standard.string(forKey: "serverURL") ?? "http://127.0.0.1:8080"
        appId = UserDefaults.standard.string(forKey: "appId") ?? "TEAMID.cloud.deltasync.capture"
    }

    func prepare() async {
        await run("Preparing the device key…") {
            guard let base = URL(string: self.serverURL) else {
                throw AuthorityError.transport("The server URL is not valid.")
            }
            let identity = try DeviceIdentity.loadOrCreate(allowSoftwareFallback: self.allowSoftwareKey)
            self.identity = identity
            self.backing = identity.backing.rawValue
            let signer = SecureEnclaveDeviceSigner(privateKey: identity.privateKey)
            let csr = try signer.makeCsr(commonName: "DeltaSync device")
            let point = try signer.publicKeyPoint()
            let authority = SigningAuthority(baseURL: base)
            let challenge = try await authority.challenge()
            var keyId: String?
            var attestation: Data?
            if identity.backing == .secureEnclave {
                let attested = try await authority.attest(challenge: challenge, uncompressedPoint: point)
                keyId = attested.keyId
                attestation = attested.object
            }
            let issued = try await authority.issue(
                challenge: challenge,
                csrPem: csr,
                appId: self.appId.trimmingCharacters(in: .whitespaces),
                keyId: keyId,
                attestation: attestation
            )
            signer.setCertificateChain(issued.certificateChainPem)
            self.signer = signer
            self.status = "\(identity.backing.rawValue). Certificate expires \(issued.expiresAt). \(issued.attestation.detail)"
        }
    }

    func captureAndSign() async {
        await run("Capturing…") {
            guard let identity = self.identity, let signer = self.signer else {
                throw ManifestError.signing("Prepare the device before capturing.")
            }
            try await self.camera.start()
            let jpeg = try await self.camera.capture()
            self.status = "Signing the capture…"
            let signed = try ManifestSigner.signJpeg(jpeg, signer: signer)
            self.signedPhoto = signed
            self.status = "Signed \(signed.count) bytes with the \(identity.backing.rawValue) key."
        }
    }

    private func run(_ start: String, _ work: @escaping () async throws -> Void) async {
        busy = true
        status = start
        defer { busy = false }
        do {
            try await work()
        } catch {
            status = error.localizedDescription
        }
    }
}
