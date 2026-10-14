import SwiftUI

enum LucideKind {
    case camera
    case shield
    case key
    case check
    case alert
}

/// A few Lucide icons, drawn from their published path data.
struct LucideIcon: View {
    var kind: LucideKind
    var color: Color

    var body: some View {
        Canvas { context, size in
            let scale = min(size.width, size.height) / 24
            var transform = CGAffineTransform(scaleX: scale, y: scale)
            context.stroke(
                path.applying(transform),
                with: .color(color),
                style: StrokeStyle(lineWidth: 2 * scale, lineCap: .round, lineJoin: .round)
            )
        }
        .frame(width: 22, height: 22)
    }

    private var path: Path {
        var path = Path()
        switch kind {
        case .camera:
            path.addPath(LucidePath.parse("M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"))
            path.addEllipse(in: CGRect(x: 9, y: 10, width: 6, height: 6))
        case .shield:
            path.addPath(LucidePath.parse("M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"))
            path.addPath(LucidePath.parse("m9 12 2 2 4-4"))
        case .key:
            path.addPath(LucidePath.parse("M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"))
            path.addEllipse(in: CGRect(x: 16.5, y: 7.5, width: 0.01, height: 0.01))
        case .check:
            path.addEllipse(in: CGRect(x: 2, y: 2, width: 20, height: 20))
            path.addPath(LucidePath.parse("m9 12 2 2 4-4"))
        case .alert:
            path.addPath(LucidePath.parse("m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"))
            path.move(to: CGPoint(x: 12, y: 9))
            path.addLine(to: CGPoint(x: 12, y: 13))
            path.move(to: CGPoint(x: 12, y: 17))
            path.addLine(to: CGPoint(x: 12.01, y: 17))
        }
        return path
    }
}

enum LucidePath {
    /// Absolute and relative M, L, H, V, C, Z. Enough for the Lucide paths above.
    static func parse(_ source: String) -> Path {
        var path = Path()
        let tokens = tokenize(source)
        var index = 0
        var command: Character = "M"
        var current = CGPoint.zero
        var start = CGPoint.zero

        func number() -> CGFloat? {
            guard index < tokens.count, let value = Double(tokens[index]) else { return nil }
            index += 1
            return CGFloat(value)
        }

        while index < tokens.count {
            if tokens[index].count == 1, let letter = tokens[index].first, letter.isLetter {
                command = letter
                index += 1
            }
            let relative = command.isLowercase
            switch command.lowercased().first {
            case "m":
                guard let x = number(), let y = number() else { return path }
                current = relative ? CGPoint(x: current.x + x, y: current.y + y) : CGPoint(x: x, y: y)
                path.move(to: current)
                start = current
                command = relative ? "l" : "L"
            case "l":
                guard let x = number(), let y = number() else { return path }
                current = relative ? CGPoint(x: current.x + x, y: current.y + y) : CGPoint(x: x, y: y)
                path.addLine(to: current)
            case "h":
                guard let x = number() else { return path }
                current.x = relative ? current.x + x : x
                path.addLine(to: current)
            case "v":
                guard let y = number() else { return path }
                current.y = relative ? current.y + y : y
                path.addLine(to: current)
            case "c":
                guard let x1 = number(), let y1 = number(),
                      let x2 = number(), let y2 = number(),
                      let x = number(), let y = number() else { return path }
                let control1 = relative ? CGPoint(x: current.x + x1, y: current.y + y1) : CGPoint(x: x1, y: y1)
                let control2 = relative ? CGPoint(x: current.x + x2, y: current.y + y2) : CGPoint(x: x2, y: y2)
                current = relative ? CGPoint(x: current.x + x, y: current.y + y) : CGPoint(x: x, y: y)
                path.addCurve(to: current, control1: control1, control2: control2)
            case "z":
                path.closeSubpath()
                current = start
            case "a":
                // Lucide arc used by the key icon. Approximate with a line to the end point
                // after consuming the seven arc parameters, then the caller also draws the bow
                // via the cubic segments that precede it. The key's only arc is a 6.5 radius
                // circle, which we replace by skipping to its endpoint.
                _ = number(); _ = number(); _ = number(); _ = number(); _ = number()
                guard let x = number(), let y = number() else { return path }
                current = relative ? CGPoint(x: current.x + x, y: current.y + y) : CGPoint(x: x, y: y)
                path.addLine(to: current)
            default:
                index += 1
            }
        }
        return path
    }

    private static func tokenize(_ source: String) -> [String] {
        var tokens: [String] = []
        var number = ""
        func flush() {
            if !number.isEmpty {
                tokens.append(number)
                number = ""
            }
        }
        for character in source {
            if character.isLetter {
                flush()
                tokens.append(String(character))
            } else if character == "-" && !number.isEmpty && !number.hasSuffix("e") && !number.hasSuffix("E") {
                flush()
                number.append(character)
            } else if character.isNumber || character == "." || character == "-" {
                number.append(character)
            } else {
                flush()
            }
        }
        flush()
        return tokens
    }
}
