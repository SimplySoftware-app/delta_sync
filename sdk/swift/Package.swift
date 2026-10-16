// swift-tools-version:5.9
import PackageDescription

// Build the binary target and the generated bindings first:
//   sdk/swift/scripts/build-xcframework.sh
let package = Package(
    name: "DeltaSyncCore",
    platforms: [.iOS(.v15)],
    products: [
        .library(name: "DeltaSyncCore", targets: ["DeltaSyncCore"]),
    ],
    targets: [
        .binaryTarget(
            name: "DeltaSyncCoreFFI",
            path: "DeltaSyncCoreFFI.xcframework"
        ),
        .target(
            name: "DeltaSyncCore",
            dependencies: ["DeltaSyncCoreFFI"],
            path: "Sources/DeltaSyncCore"
        ),
        .testTarget(
            name: "DeltaSyncCoreTests",
            dependencies: ["DeltaSyncCore"],
            path: "Tests/DeltaSyncCoreTests"
        ),
    ]
)
