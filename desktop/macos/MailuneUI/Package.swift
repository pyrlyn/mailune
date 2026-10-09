// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MailuneUI",
    platforms: [.macOS("26.0")],
    products: [
        .library(name: "MailuneUI", targets: ["MailuneUI"])
    ],
    dependencies: [
        .package(path: "../MailuneModel"),
    ],
    targets: [
        .target(name: "MailuneUI", dependencies: ["MailuneModel"]),
        .testTarget(name: "MailuneUITests", dependencies: ["MailuneUI"]),
    ]
)
