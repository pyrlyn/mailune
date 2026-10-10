#!/bin/sh
# Checks scripts/release.sh on an app that is already built: the DMG holds the
# app and an Applications link, the appcast matches the DMG, and without a
# signing identity no Apple tool runs. codesign, xcrun, notarytool, altool and
# curl are shadowed by stubs that record any call, so a run cannot reach Apple.
#
#   scripts/release_test.sh path/to/Mailune.app
set -eu
cd "$(dirname "$0")/.."

app=$1
work=$(mktemp -d)
mount="$work/mount"
trap 'hdiutil detach -quiet "$mount" 2>/dev/null || true; rm -rf "$work"' EXIT

fail() {
    echo "release_test: $*" >&2
    exit 1
}

mkdir -p "$work/bin"
for tool in codesign xcrun notarytool altool curl; do
    printf '#!/bin/sh\necho "%s $*" >> "%s/calls"\nexit 1\n' "$tool" "$work" > "$work/bin/$tool"
    chmod +x "$work/bin/$tool"
done

env -u MAILUNE_SIGN_IDENTITY PATH="$work/bin:$PATH" \
    scripts/release.sh --app "$app" --out "$work/out" > "$work/log"
grep -q "no signing identity" "$work/log" || fail "a missing identity was not reported"
[ ! -s "$work/calls" ] || fail "an Apple tool ran without an identity: $(cat "$work/calls")"

version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")
dmg="$work/out/Mailune-$version.dmg"
[ -f "$dmg" ] || fail "no Mailune-$version.dmg"

mkdir "$mount"
hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mount" "$dmg"
[ -x "$mount/Mailune.app/Contents/MacOS/Mailune" ] || fail "the DMG holds no Mailune.app"
[ "$(readlink "$mount/Applications")" = /Applications ] || fail "the DMG has no Applications link"
[ "$(ls -A "$mount" | grep -v '^\.' | sort | tr '\n' ' ')" = "Applications Mailune.app " ] \
    || fail "the DMG holds more than the app and the link"
hdiutil detach -quiet "$mount"

python3 - "$work/out/appcast.xml" "$dmg" "$version" <<'EOF' || fail "the appcast does not match the DMG"
import os, sys
import xml.etree.ElementTree as ET

appcast, dmg, version = sys.argv[1:]
sparkle = "{http://www.andymatuschak.org/xml-namespaces/sparkle}"
item = ET.parse(appcast).find("channel/item")
enclosure = item.find("enclosure")
assert item.findtext(sparkle + "shortVersionString") == version
assert enclosure.get("url").endswith("/Mailune-%s.dmg" % version)
assert int(enclosure.get("length")) == os.path.getsize(dmg)
assert enclosure.get(sparkle + "edSignature") == "UNSIGNED-FIXTURE"
EOF
echo "release_test: ok"
