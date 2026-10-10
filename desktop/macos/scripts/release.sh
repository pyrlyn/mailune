#!/bin/sh
# The macOS release layout, built locally: Mailune-X.Y.Z.dmg holding the app and
# an Applications link (the layout pyrlyn/ci release-apple-desktop.yml makes),
# and a Sparkle appcast fixture beside it.
#
# Nothing here notarizes; the release workflow does that with the org's keys.
# Without MAILUNE_SIGN_IDENTITY nothing is signed and nothing reaches Apple:
# codesign with an identity asks Apple's timestamp server, so it is skipped too.
#
#   scripts/release.sh [--app path/to/Mailune.app] [--out dir]
#
# Without --app it builds the Release configuration first.
set -eu
cd "$(dirname "$0")/.."

app=""
out="build/release"
while [ $# -gt 0 ]; do
    case "$1" in
        --app) app=$2; shift 2 ;;
        --out) out=$2; shift 2 ;;
        *) echo "usage: scripts/release.sh [--app path/to/Mailune.app] [--out dir]" >&2; exit 2 ;;
    esac
done

if [ -z "$app" ]; then
    mise exec xcodegen@2.46.0 -- xcodegen generate --quiet
    xcodebuild build \
        -project Mailune.xcodeproj \
        -scheme Mailune \
        -configuration Release \
        -destination 'platform=macOS,arch=arm64' \
        -derivedDataPath DerivedData \
        -quiet
    app=DerivedData/Build/Products/Release/Mailune.app
fi
if [ ! -d "$app" ]; then
    echo "error: no app at $app" >&2
    exit 1
fi

plist="$app/Contents/Info.plist"
read_plist() {
    /usr/libexec/PlistBuddy -c "Print :$1" "$plist"
}
version=$(read_plist CFBundleShortVersionString)
build=$(read_plist CFBundleVersion)
minimum=$(read_plist LSMinimumSystemVersion)

identity=${MAILUNE_SIGN_IDENTITY:-}
if [ -z "$identity" ]; then
    echo "no signing identity: MAILUNE_SIGN_IDENTITY is unset, so the app and the DMG stay unsigned and nothing is sent to Apple"
fi

mkdir -p "$out"
stage="$out/dmg"
rm -rf "$stage"
mkdir -p "$stage"
ditto "$app" "$stage/Mailune.app"
ln -s /Applications "$stage/Applications"
if [ -n "$identity" ]; then
    codesign --force --options runtime --timestamp --sign "$identity" "$stage/Mailune.app"
fi

dmg="$out/Mailune-$version.dmg"
rm -f "$dmg"
hdiutil create -quiet -volname Mailune -srcfolder "$stage" -fs HFS+ -format UDZO "$dmg"
hdiutil verify -quiet "$dmg"
if [ -n "$identity" ]; then
    codesign --sign "$identity" --timestamp "$dmg"
fi
echo "not notarized: the release workflow notarizes and staples with the org's App Store Connect key"

# A fixture, not a feed: the EdDSA signature is a placeholder that Sparkle
# rejects, because only the release workflow holds the Sparkle key.
length=$(stat -f %z "$dmg")
published=$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S +0000')
cat > "$out/appcast.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Mailune</title>
    <item>
      <title>$version</title>
      <pubDate>$published</pubDate>
      <sparkle:version>$build</sparkle:version>
      <sparkle:shortVersionString>$version</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>$minimum</sparkle:minimumSystemVersion>
      <enclosure url="https://github.com/pyrlyn/mailune/releases/download/desktop-v$version/Mailune-$version.dmg" length="$length" type="application/octet-stream" sparkle:edSignature="UNSIGNED-FIXTURE"/>
    </item>
  </channel>
</rss>
EOF
echo "wrote $dmg and $out/appcast.xml"
