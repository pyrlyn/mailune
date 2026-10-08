// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MailuneUI",
    defaultLocalization: "en",
    platforms: [.macOS("26.0"), .iOS("26.0")],
    products: [
        .library(name: "MailuneUI", targets: ["MailuneUI"])
    ],
    dependencies: [
        .package(path: "../MailuneModel"),
    ],
    targets: [
        .target(
            name: "MailuneUI",
            dependencies: ["MailuneModel"],
            resources: [
                .process("en.lproj"),
                .process("en-GB.lproj"),
            ]
        ),
        .testTarget(name: "MailuneUITests", dependencies: ["MailuneUI"]),
    ]
)
