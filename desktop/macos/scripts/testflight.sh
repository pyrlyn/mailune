#!/usr/bin/env bash
# Prints the TestFlight caller. Nothing here uploads a build or contacts Apple.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
caller="$root/testflight.yml"

if [[ "${1:-}" == "--upload" || "${MAILUNE_TESTFLIGHT_UPLOAD:-}" == "1" ]]; then
  echo "upload is not part of this script" >&2
  exit 2
fi

if [[ ! -f "$caller" ]]; then
  echo "missing TestFlight caller description" >&2
  exit 1
fi

echo "caller description: $caller"
echo "would call pyrlyn/ci ios-testflight.yml for scheme MailuneIOS"
echo "not uploading"
echo "not notarising"
