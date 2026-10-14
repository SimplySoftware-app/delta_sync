import CryptoKit
import DeviceCheck
import Foundation

struct AttestationOutcome: Decodable {
    let mode: String
    let verified: Bool
    let detail: String
}

struct IssuedCertificate: Decodable {
    let certificatePem: String
    let certificateChainPem: String
    let expiresAt: String
    let attestation: AttestationOutcome

    enum CodingKeys: String, CodingKey {
        case certificatePem = "certificate_pem"
        case certificateChainPem = "certificate_chain_pem"
        case expiresAt = "expires_at"
        case attestation
    }
}

enum AuthorityError: Error, LocalizedError {
    case transport(String)
    case server(Int, String)
    case appAttest(String)

    var errorDescription: String? {
        switch self {
        case .transport(let message), .appAttest(let message):
            return message
        case .server(let code, let message):
            return "Signing authority returned \(code): \(message)"
        }
    }
}

struct SigningAuthority {
    var baseURL: URL

    func challenge(platform: String = "ios") async throws -> String {
        struct Body: Decodable { let challenge: String }
        let body: Body = try await send(
            path: "/v1/challenge",
            json: ["platform": platform]
        )
        return body.challenge
    }

    func issue(
        challenge: String,
        csrPem: String,
        appId: String,
        keyId: String?,
        attestation: Data?
    ) async throws -> IssuedCertificate {
        var payload: [String: String] = [
            "platform": "ios",
            "challenge": challenge,
            "csr_pem": csrPem,
            "app_id": appId
        ]
        if let keyId {
            payload["key_id"] = keyId
        }
        if let attestation {
            payload["attestation"] = attestation.base64EncodedString()
        }
        return try await send(path: "/v1/device-certificate", json: payload)
    }

    func attest(challenge: String, uncompressedPoint: Data) async throws -> (keyId: String, object: Data) {
        let service = DCAppAttestService.shared
        guard service.isSupported else {
            throw AuthorityError.appAttest(
                "App Attest is not available. It requires a physical iPhone and the App Attest capability on this App ID."
            )
        }
        let keyId = try await service.generateKey()
        var material = Data(challenge.utf8)
        material.append(0)
        material.append(uncompressedPoint)
        let clientDataHash = Data(SHA256.hash(data: material))
        let object = try await service.attestKey(keyId, clientDataHash: clientDataHash)
        return (keyId, object)
    }

    private func endpoint(_ path: String) throws -> URL {
        guard var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false) else {
            throw AuthorityError.transport("The server URL is not valid.")
        }
        components.path = path
        guard let url = components.url else {
            throw AuthorityError.transport("The server URL is not valid.")
        }
        return url
    }

    private func send<Response: Decodable>(path: String, json: [String: String]) async throws -> Response {
        var request = URLRequest(url: try endpoint(path))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: json)
        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await URLSession.shared.data(for: request)
        } catch {
            throw AuthorityError.transport(error.localizedDescription)
        }
        let http = response as? HTTPURLResponse
        let status = http?.statusCode ?? 0
        guard (200..<300).contains(status) else {
            let message = (try? JSONDecoder().decode(ErrorBody.self, from: data))?.error
                ?? String(data: data, encoding: .utf8)
                ?? "empty response"
            throw AuthorityError.server(status, message)
        }
        do {
            return try JSONDecoder().decode(Response.self, from: data)
        } catch {
            throw AuthorityError.transport("Could not read the signing authority response.")
        }
    }
}

private struct ErrorBody: Decodable {
    let error: String
}
