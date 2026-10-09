// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "MailunePlatform",
    platforms: [.macOS("26.0")],
    products: [
        .library(name: "MailunePlatform", targets: ["MailunePlatform"])
    ],
    dependencies: [
        .package(path: "../MailuneModel"),
    ],
    targets: [
        .target(name: "MailunePlatform", dependencies: ["MailuneModel"]),
        .testTarget(name: "MailunePlatformTests", dependencies: ["MailunePlatform"]),
    ]
)
