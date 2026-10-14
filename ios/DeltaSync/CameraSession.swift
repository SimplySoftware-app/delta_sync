import AVFoundation
import UIKit

enum CameraError: Error, LocalizedError {
    case permission
    case unavailable
    case capture(String)

    var errorDescription: String? {
        switch self {
        case .permission:
            return "Camera permission is required to capture a photo."
        case .unavailable:
            return "No camera is available."
        case .capture(let message):
            return message
        }
    }
}

final class CameraSession: NSObject, AVCapturePhotoCaptureDelegate {
    let session = AVCaptureSession()
    private let output = AVCapturePhotoOutput()
    private var continuation: CheckedContinuation<Data, Error>?

    func start() async throws {
        if session.isRunning { return }
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            break
        case .notDetermined:
            let granted = await AVCaptureDevice.requestAccess(for: .video)
            if !granted { throw CameraError.permission }
        default:
            throw CameraError.permission
        }
        session.beginConfiguration()
        session.sessionPreset = .photo
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device),
              session.canAddInput(input),
              session.canAddOutput(output) else {
            session.commitConfiguration()
            throw CameraError.unavailable
        }
        session.addInput(input)
        session.addOutput(output)
        session.commitConfiguration()
        if !session.isRunning {
            session.startRunning()
        }
    }

    func capture() async throws -> Data {
        try await withCheckedThrowingContinuation { continuation in
            self.continuation = continuation
            let settings = AVCapturePhotoSettings()
            if output.availablePhotoCodecTypes.contains(.jpeg) {
                settings.photoQualityPrioritization = .balanced
            }
            output.capturePhoto(with: settings, delegate: self)
        }
    }

    func photoOutput(
        _ output: AVCapturePhotoOutput,
        didFinishProcessingPhoto photo: AVCapturePhoto,
        error: Error?
    ) {
        if let error {
            continuation?.resume(throwing: CameraError.capture(error.localizedDescription))
        } else if let data = photo.fileDataRepresentation() {
            continuation?.resume(returning: data)
        } else {
            continuation?.resume(throwing: CameraError.capture("The camera did not return a JPEG."))
        }
        continuation = nil
    }
}
