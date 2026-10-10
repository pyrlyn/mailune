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
# The release layout from the Debug app, with no identity, so nothing reaches Apple.
scripts/release_test.sh DerivedData/Build/Products/Debug/Mailune.app

xcodebuild build \
    -project Mailune.xcodeproj \
    -scheme MailuneIOS \
    -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath DerivedData \
    -quiet
only_arm64 DerivedData/Build/Products/Debug-iphonesimulator/MailuneIOS.app/MailuneIOS

# The phone UI test only runs in scripts/uitest.sh, but it must still compile here.
xcodebuild build-for-testing \
    -project Mailune.xcodeproj \
    -scheme MailuneIOSUITests \
    -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath DerivedData \
    -quiet

simulator=$(scripts/simulator.sh)
for package in MailuneModel MailuneUI; do
    (cd "$package" && xcodebuild test \
        -scheme "$package" \
        -destination "id=$simulator" \
        -derivedDataPath ../DerivedData-ios \
        -quiet)
done
