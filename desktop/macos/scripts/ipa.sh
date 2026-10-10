#!/bin/sh
# Archives MailuneIOS for devices and exports an App Store Connect IPA to
# build/ipa/MailuneIOS.ipa. It is the build command of .github/workflows/
# testflight.yml, which hands the IPA to the R13 TestFlight workflow; this
# script never uploads.
#
# Signing is Xcode's automatic, cloud-managed signing, so no certificate has to
# sit in a keychain: the archive is signed for development and the export
# re-signs it for App Store Connect. It needs MAILUNE_TEAM_ID and an App Store Connect API key:
# APPSTORE_CONNECT_KEY_ID, APPSTORE_CONNECT_ISSUER_ID, and the key at
# ~/private_keys/AuthKey_<id>.p8, where altool reads it too. Without them the
# script says what is missing and stops before anything talks to Apple.
set -eu
cd "$(dirname "$0")/.."

team=${MAILUNE_TEAM_ID:-}
key_id=${APPSTORE_CONNECT_KEY_ID:-}
issuer=${APPSTORE_CONNECT_ISSUER_ID:-}
key="$HOME/private_keys/AuthKey_$key_id.p8"

missing=""
[ -n "$team" ] || missing="$missing MAILUNE_TEAM_ID"
[ -n "$key_id" ] || missing="$missing APPSTORE_CONNECT_KEY_ID"
[ -n "$issuer" ] || missing="$missing APPSTORE_CONNECT_ISSUER_ID"
[ -z "$key_id" ] || [ -f "$key" ] || missing="$missing $key"
if [ -n "$missing" ]; then
    echo "no signing identity: missing$missing, so no IPA is made and nothing is sent to Apple" >&2
    exit 1
fi
if ! printf '%s' "$team" | grep -Eq '^[A-Z0-9]{10}$'; then
    echo "error: MAILUNE_TEAM_ID is not a ten-character team id" >&2
    exit 1
fi

archive=build/MailuneIOS.xcarchive
rm -rf "$archive" build/ipa
mise exec xcodegen@2.46.0 -- xcodegen generate --quiet
xcodebuild archive \
    -project Mailune.xcodeproj \
    -scheme MailuneIOS \
    -configuration Release \
    -destination 'generic/platform=iOS' \
    -archivePath "$archive" \
    -derivedDataPath DerivedData-ipa \
    -allowProvisioningUpdates \
    -authenticationKeyPath "$key" \
    -authenticationKeyID "$key_id" \
    -authenticationKeyIssuerID "$issuer" \
    DEVELOPMENT_TEAM="$team" \
    CODE_SIGN_STYLE=Automatic \
    CODE_SIGN_IDENTITY="Apple Development" \
    -quiet

options=$(mktemp)
trap 'rm -f "$options"' EXIT
cp ExportOptions-AppStore.plist "$options"
/usr/libexec/PlistBuddy -c "Set :teamID $team" "$options"
xcodebuild -exportArchive \
    -archivePath "$archive" \
    -exportOptionsPlist "$options" \
    -exportPath build/ipa \
    -allowProvisioningUpdates \
    -authenticationKeyPath "$key" \
    -authenticationKeyID "$key_id" \
    -authenticationKeyIssuerID "$issuer" \
    -quiet
[ -f build/ipa/MailuneIOS.ipa ] || { echo "error: the export made no build/ipa/MailuneIOS.ipa" >&2; exit 1; }
echo "wrote build/ipa/MailuneIOS.ipa"
