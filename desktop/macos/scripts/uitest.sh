#!/bin/sh
# The XCUITest scheme. It needs a logged-in GUI session, so it is not part of
# scripts/test.sh, which only compiles it.
set -eu
cd "$(dirname "$0")/.."

mise exec xcodegen@2.46.0 -- xcodegen generate --quiet
xcodebuild test \
    -project Mailune.xcodeproj \
    -scheme MailuneUITests \
    -destination 'platform=macOS,arch=arm64' \
    -derivedDataPath DerivedData
