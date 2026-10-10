#!/bin/sh
# Rebuilds the S/MIME fixtures with OpenSSL, so the tests check interop with
# an implementation other than our own. The keys are test-only and public.
set -eu
cd "$(dirname "$0")"
for who in ada bob; do
  openssl req -x509 -newkey rsa:2048 -nodes -days 36500 -sha256 \
    -keyout "$who.key.pem" -out "$who.crt.pem" \
    -subj "/CN=$who/emailAddress=$who@example.com" \
    -addext "subjectKeyIdentifier=hash" \
    -addext "keyUsage=digitalSignature,keyEncipherment" \
    -addext "extendedKeyUsage=emailProtection" 2>/dev/null
done
printf 'Content-Type: text/plain; charset=utf-8\r\n\r\nHello from Ada.\r\n' > content.eml
openssl cms -sign -binary -md sha256 -in content.eml -signer ada.crt.pem -inkey ada.key.pem \
  -outform DER -out signed-detached.p7s
openssl cms -sign -binary -nodetach -keyid -md sha256 -in content.eml -signer ada.crt.pem \
  -inkey ada.key.pem -outform DER -out signed-opaque-keyid.p7m
openssl cms -encrypt -binary -aes256 -in content.eml -outform DER -out enveloped-aes256.p7m ada.crt.pem
openssl cms -encrypt -binary -aes128 -in content.eml -outform DER -out enveloped-aes128.p7m \
  bob.crt.pem ada.crt.pem
