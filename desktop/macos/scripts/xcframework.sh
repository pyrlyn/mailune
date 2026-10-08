#!/usr/bin/env bash
# Builds mailune-ffi into desktop/macos/MailuneCore: one arm64 macOS static
# library inside an XCFramework, and the Swift bindings UniFFI reads from that
# library. Both are build output. This script does not build iOS or Intel.

set -euo pipefail

cd "$(dirname "$0")/../../.."
root=$(pwd)

[ "$(uname -s)" = Darwin ] || { echo "an XCFramework is built on macOS" >&2; exit 1; }
[ "$(uname -m)" = arm64 ] || { echo "macOS builds are arm64 only" >&2; exit 1; }

export MACOSX_DEPLOYMENT_TARGET=26.0

package="$root/desktop/macos/MailuneCore"
work="$root/target/xcframework"
target=aarch64-apple-darwin

cargo build --locked -p mailune-ffi --lib --target "$target"
lib="$root/target/$target/debug/libmailune_ffi.a"

rm -rf "$work"
mkdir -p "$work/include"
cp "$lib" "$work/libmailune_ffi.a"

cargo run --locked -q -p mailune-ffi --features bindgen --bin uniffi-bindgen -- \
    generate --library "$lib" --language swift --out-dir "$work/swift"

cp "$work/swift/mailune_ffiFFI.h" "$work/include/"
cp "$work/swift/mailune_ffiFFI.modulemap" "$work/include/module.modulemap"

rm -rf "$package/MailuneFFI.xcframework"
xcodebuild -create-xcframework \
    -library "$work/libmailune_ffi.a" -headers "$work/include" \
    -output "$package/MailuneFFI.xcframework" >/dev/null

mkdir -p "$package/Sources/MailuneCore"
cp "$work/swift/mailune_ffi.swift" "$package/Sources/MailuneCore/mailune_ffi.swift"

echo "built $package/MailuneFFI.xcframework"
