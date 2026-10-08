// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MailuneUI",
    platforms: [.macOS("26.0")],
    products: [
        .library(name: "MailuneUI", targets: ["MailuneUI"])
    ],
    targets: [
        .target(name: "MailuneUI"),
    ]
)
