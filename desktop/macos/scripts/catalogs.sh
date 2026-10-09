#!/bin/sh
# Copies the Apple catalogs that scripts/i18n.py generates from i18n/*.po into
# MailuneUI. The copies are build output and git-ignored, because i18n/ is the
# only place a string is written. SwiftPM cannot reach target/i18n directly:
# a resource must sit inside its target's folder.
set -eu
macos=$(cd "$(dirname "$0")/.." && pwd)
root=$(cd "$macos/../.." && pwd)

(cd "$root" && mise run i18n)

dest="$macos/MailuneUI/Sources/MailuneUI/Catalogs"
rm -rf "$dest"
mkdir -p "$dest"
cp -R "$root"/target/i18n/apple/*.lproj "$dest"/
