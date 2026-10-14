import SwiftUI
import UIKit

struct ContentView: View {
    @StateObject private var model = CaptureModel()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                header
                field("Signing authority", text: $model.serverURL)
                field("App ID (TeamID.bundle)", text: $model.appId)
                #if DEBUG
                Toggle(isOn: $model.allowSoftwareKey) {
                    Label {
                        Text("Simulator software key")
                    } icon: {
                        LucideIcon(kind: .key, color: DeltaColor.clay)
                    }
                    .font(.subheadline)
                }
                .tint(DeltaColor.navy)
                Text("This key is not in the Secure Enclave. The server must be in stub mode, and the photo is not a hardware capture proof.")
                    .font(.footnote)
                    .foregroundStyle(DeltaColor.ink.opacity(0.7))
                #endif
                statusCard
                Button(action: { Task { await model.prepare() } }) {
                    Label("Prepare device", systemImage: "checkmark")
                        .labelStyle(.titleAndIcon)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(DeltaButtonStyle())
                .disabled(model.busy)
                .overlay(alignment: .leading) {
                    LucideIcon(kind: .shield, color: .white)
                        .padding(.leading, 18)
                        .allowsHitTesting(false)
                }
                Button(action: { Task { await model.captureAndSign() } }) {
                    Text("Capture and sign")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(DeltaButtonStyle(filled: false))
                .disabled(model.busy)
                .overlay(alignment: .leading) {
                    LucideIcon(kind: .camera, color: DeltaColor.navy)
                        .padding(.leading, 18)
                        .allowsHitTesting(false)
                }
                if let photo = model.signedPhoto, let image = UIImage(data: photo) {
                    Image(uiImage: image)
                        .resizable()
                        .scaledToFit()
                        .frame(maxHeight: 280)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                    ShareLink(item: photoFile(photo), preview: SharePreview("Signed capture")) {
                        Label("Share signed JPEG", systemImage: "square.and.arrow.up")
                    }
                }
            }
            .padding(20)
        }
        .background(DeltaColor.paper.ignoresSafeArea())
    }

    private var header: some View {
        HStack(spacing: 12) {
            LucideIcon(kind: .shield, color: DeltaColor.navy)
                .frame(width: 28, height: 28)
            VStack(alignment: .leading, spacing: 2) {
                Text("DeltaSync")
                    .font(.title2.bold())
                    .foregroundStyle(DeltaColor.navy)
                Text("Sign a photo as it is captured.")
                    .font(.subheadline)
                    .foregroundStyle(DeltaColor.ink.opacity(0.75))
            }
        }
    }

    private func field(_ title: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption.weight(.semibold))
                .foregroundStyle(DeltaColor.navy)
            TextField(title, text: text)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .padding(12)
                .background(Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 10))
        }
    }

    private var statusCard: some View {
        HStack(alignment: .top, spacing: 10) {
            LucideIcon(kind: model.signedPhoto == nil ? .alert : .check, color: DeltaColor.navy)
            Text(model.status)
                .font(.subheadline)
                .foregroundStyle(DeltaColor.ink)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(14)
        .background(Color.white)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private func photoFile(_ data: Data) -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("deltasync-share.jpg")
        try? data.write(to: url, options: .atomic)
        return url
    }
}

enum DeltaColor {
    static let paper = Color(red: 238 / 255, green: 233 / 255, blue: 223 / 255)
    static let ink = Color(red: 28 / 255, green: 28 / 255, blue: 30 / 255)
    static let navy = Color(red: 44 / 255, green: 59 / 255, blue: 77 / 255)
    static let clay = Color(red: 163 / 255, green: 81 / 255, blue: 57 / 255)
}

struct DeltaButtonStyle: ButtonStyle {
    var filled = true

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.body.weight(.semibold))
            .padding(.vertical, 14)
            .padding(.horizontal, 16)
            .foregroundStyle(filled ? Color.white : DeltaColor.navy)
            .background(filled ? DeltaColor.navy : Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(DeltaColor.navy, lineWidth: filled ? 0 : 1)
            )
            .opacity(configuration.isPressed ? 0.8 : 1)
    }
}
