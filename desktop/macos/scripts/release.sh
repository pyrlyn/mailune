#!/usr/bin/env bash
# Stages a DMG layout and an appcast fixture. Signing is local only.
# When MAILUNE_SIGN_IDENTITY is unset or "-", the script says so and returns
# without invoking notarization.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/dist"
stage="$dist/dmg"
app="$stage/Mailune.app"

rm -rf "$stage"
mkdir -p "$app/Contents/MacOS"

cat > "$app/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>app.mailune.macos</string>
  <key>CFBundleName</key>
  <string>Mailune</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
</dict>
</plist>
EOF

printf '#!/bin/sh\nexit 0\n' > "$app/Contents/MacOS/Mailune"
chmod +x "$app/Contents/MacOS/Mailune"
ln -sfn /Applications "$stage/Applications"

cat > "$dist/appcast.xml" << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
  <channel>
    <title>Mailune</title>
    <item>
      <title>0.1.0</title>
      <enclosure url="Mailune-0.1.0.dmg" length="0" type="application/octet-stream"/>
    </item>
  </channel>
</rss>
EOF

echo "dmg layout: $stage"
echo "appcast fixture: $dist/appcast.xml"

identity="${MAILUNE_SIGN_IDENTITY:-}"
if [[ -z "$identity" || "$identity" == "-" ]]; then
  echo "signing identity is missing; not contacting Apple"
  exit 0
fi

echo "signing identity is set; notarization is not part of this script"
