#!/bin/sh
# Every Apple-shell test: the three local packages on macOS, the generated Xcode
# project on the arm64 slice, then the iOS app for the simulator and the shared
# packages' tests on an iPhone simulator. XcodeGen is pinned here rather than in
# mise.toml because it has no Linux build and mise.toml is installed on Linux CI.
set -eu
cd "$(dirname "$0")/.."

only_arm64() {
    slices=$(lipo -archs "$1")
    if [ "$slices" != "arm64" ]; then
        echo "error: $1 has slices '$slices'; only arm64 is supported" >&2
        exit 1
    fi
}

scripts/catalogs.sh
for package in MailuneModel MailuneUI MailunePlatform; do
    swift test --package-path "$package"
done

mise exec xcodegen@2.46.0 -- xcodegen generate --quiet
xcodebuild test \
    -project Mailune.xcodeproj \
    -scheme Mailune \
    -destination 'platform=macOS,arch=arm64' \
    -derivedDataPath DerivedData \
    -quiet

# The UI test only runs in scripts/uitest.sh, but it must still compile here.
xcodebuild build-for-testing \
    -project Mailune.xcodeproj \
    -scheme MailuneUITests \
    -destination 'platform=macOS,arch=arm64' \
    -derivedDataPath DerivedData \
    -quiet
only_arm64 DerivedData/Build/Products/Debug/Mailune.app/Contents/MacOS/Mailune

xcodebuild build \
    -project Mailune.xcodeproj \
    -scheme MailuneIOS \
    -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath DerivedData \
    -quiet
only_arm64 DerivedData/Build/Products/Debug-iphonesimulator/MailuneIOS.app/MailuneIOS

# The first iPhone on the newest installed iOS runtime, so the script works on
# whichever simulators a machine or runner image ships.
simulator=$(xcrun simctl list devices available --json | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
runtimes = sorted(
    (key for key in devices if ".SimRuntime.iOS-" in key),
    key=lambda key: [int(part) for part in key.rsplit("iOS-", 1)[1].split("-")],
)
for runtime in reversed(runtimes):
    for device in devices[runtime]:
        if device["name"].startswith("iPhone"):
            print(device["udid"])
            sys.exit(0)
sys.exit("no iPhone simulator is installed")
')
for package in MailuneModel MailuneUI; do
    (cd "$package" && xcodebuild test \
        -scheme "$package" \
        -destination "id=$simulator" \
        -derivedDataPath ../DerivedData-ios \
        -quiet)
done
