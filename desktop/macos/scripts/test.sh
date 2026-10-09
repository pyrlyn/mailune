#!/bin/sh
# Every macOS test: the three local packages, then the generated Xcode project
# on the arm64 slice. XcodeGen is pinned here rather than in mise.toml because
# it has no Linux build and mise.toml is installed on Linux CI too.
set -eu
cd "$(dirname "$0")/.."

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

slices=$(lipo -archs DerivedData/Build/Products/Debug/Mailune.app/Contents/MacOS/Mailune)
if [ "$slices" != "arm64" ]; then
    echo "error: Mailune.app has slices '$slices'; only arm64 is supported" >&2
    exit 1
fi
