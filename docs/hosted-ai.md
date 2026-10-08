# Paid hosted AI

A later paid tier runs one cloud model inside Azure confidential containers (Azure Container Apps). The tier is another cloud model behind the privacy router that already lives in `mailune-ai`. This document does not add a provider, a socket, or a path that sends mail.

## Provider

The hosted model is a `ModelKind::Cloud` capability with id `hosted-azure-cc`. Azure confidential containers are the confidential-compute host: the inference process runs in a hardware-backed container, and the client checks that container's attestation (the measurement of the inference image) before a prompt is released. A failed or missing attestation is a refusal. The prompt stays on the device.

The person's bring-your-own-key models stay the request builders in `mailune-ai` (`openai_request`, `anthropic_request`). The hosted tier is not a new router and not a bypass of those checks. A later adapter would sit next to those builders. It is not part of this design, and it must not post mail anywhere.

## Router

`Router::route` is the only choice. The fallback chain is unchanged: platform, then a bundled local model, then cloud. Cloud is eligible only when `effective_privacy` is `PrivacyClass::CloudAllowed`.

Encrypted mail is never sent to a cloud model. `effective_privacy` forces OpenPGP and S/MIME mail to `PrivacyClass::LocalOnly` before the chain runs, so `hosted-azure-cc` is not selected. That covers ciphertext, decrypted plaintext, keys, and text extracted from those messages. Transport TLS does not count as that encryption. A local model may still handle the message on the device.

`allow_cloud` is the same gate a later adapter must call. `CloudForbidden` is the failure for local-only and local-preferred mail, including every encrypted message. There is no hosted-only exception.

Before a cloud call that the router did allow, quoted text and signatures go through `redact_for_cloud`. The call writes a `FlowRecord`: feature, provider `hosted-azure-cc`, bytes sent, and message ids. The provider name is not a secret. Prompt text is not logged.

## What the provider does not do

The hosted model only returns a completion. It has no mail transport, so it cannot send, delete, or forward. It does not receive tokens or the SQLCipher key. Those stay in the OS keychain. A typed tool call is still only a proposal: policy outside the model approves it and fails closed, and send, delete, and forward still need confirmation in the app.

The paid opt-in is per feature, the same classes as bring-your-own-key (`local-only`, `local-preferred`, `cloud-allowed`). Local remains the default.
