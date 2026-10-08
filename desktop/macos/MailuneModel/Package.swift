// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MailuneModel",
    platforms: [.macOS("26.0")],
    products: [
        .library(name: "MailuneModel", targets: ["MailuneModel"])
    ],
    targets: [
        .target(name: "MailuneModel"),
    ]
)
