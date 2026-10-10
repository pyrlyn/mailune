#!/bin/sh
# The XCUITest schemes: the Mac app, then the phone layout on an iPhone
# simulator. The Mac run needs a logged-in GUI session, so neither is part of
# scripts/test.sh, which only compiles them.
set -eu
cd "$(dirname "$0")/.."

mise exec xcodegen@2.46.0 -- xcodegen generate --quiet
xcodebuild test \
    -project Mailune.xcodeproj \
    -scheme MailuneUITests \
    -destination 'platform=macOS,arch=arm64' \
    -derivedDataPath DerivedData

xcodebuild test \
    -project Mailune.xcodeproj \
    -scheme MailuneIOSUITests \
    -destination "id=$(scripts/simulator.sh)" \
    -derivedDataPath DerivedData
