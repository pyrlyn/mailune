// swift-tools-version: 6.0

// MailuneCore: the Rust core (crates/mailune-ffi) as a Swift package.
// desktop/macos/scripts/xcframework.sh builds the arm64 XCFramework and the
// UniFFI bindings into this folder. macOS only; no iOS and no Intel slice.

import PackageDescription

let package = Package(
    name: "MailuneCore",
    platforms: [.macOS("26.0")],
    products: [
        .library(name: "MailuneCore", targets: ["MailuneCore"])
    ],
    targets: [
        .binaryTarget(name: "mailune_ffiFFI", path: "MailuneFFI.xcframework"),
        .target(
            name: "MailuneCore",
            dependencies: ["mailune_ffiFFI"],
            linkerSettings: [
                .linkedFramework("CoreFoundation"),
            ]
        ),
        .testTarget(name: "MailuneCoreTests", dependencies: ["MailuneCore"]),
    ]
)
