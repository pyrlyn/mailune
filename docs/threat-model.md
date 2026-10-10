# Threat model

This document records the assets, trust boundaries, and fail-closed rules for the Mailune core, the AI path, MCP, and the push relay, and names the code that holds each rule today. It is a draft for the creator to review. The rendering, storage, and authentication measures themselves are listed in the Security section of [`architecture.md`](architecture.md#security); this file says which boundary each one protects and does not repeat them.

The shape of the rules matches the rest of the workspace. Policy is deterministic code outside the model and fails closed. One engine allows, denies, or escalates a call, and the caller does not approve itself. Deny is decided before allow. A missing, malformed, or unknown decision is a denial. A broken MCP server or plugin is warned about and skipped; that skip is not an approval.

## Assets

| Asset | What it is | Where it may live |
| --- | --- | --- |
| Mail content | Headers, bodies, attachments, drafts, and search text | Provider servers and the encrypted local store. Treated as data everywhere else. |
| Tokens | OAuth and other provider credentials, and bring-your-own-key model keys | OS keychain only, read through the `SecretStore` host callback. |
| SQLCipher key | Key that opens the local database | OS keychain only. It is not stored in the database file. |
| Account config | Hosts, identity, privacy class, and per-feature opt-in | Local config. It does not hold tokens, passwords, or the SQLCipher key. |
| Push channel id | The random id in a relay webhook URL | The device, the relay's route table, and the provider subscription. Whoever holds it can only trigger empty wakes. |
| Model blobs | Local model weights | The app's model directory, trusted only after their digest matches the catalog. |

Mail content stays an asset after it is stored. Owning a copy does not make the bytes trusted instructions.

## Trust boundaries

The device talks to providers directly. The encrypted local database is the source of truth for the UI. Servers are the source of truth for mail state. Surfaces (`mailune-ffi`, `mailune-capi`, `mailune-rpc`, `mailune-wasm`, `mailune-cli`, `mailune-mcp`, `mailune-server`) only forward calls. They do not decide policy.

| Boundary | Trusted for | Not trusted for |
| --- | --- | --- |
| Provider servers | Account mail state, after an authenticated session | Message content as instructions. Bodies, HTML, and headers are data. |
| Local store | Our encrypted records (SQLCipher, FTS5, blobs) as the UI's copy | Turning stored mail into an instruction or a secret. The key is not in the file. |
| OS keychain | Holding the SQLCipher key and provider tokens. Android uses Keystore through the host callback. | Deciding which tool runs, or confirming send, delete, or forward. |
| Local model | Proposing typed results on device (platform model or bundled llama.cpp). This is the default. | Approving those results, or confirming send, delete, or forward. Its text is untrusted. |
| Cloud model | A bring-your-own-key provider, and only when that feature is opted in | Encrypted mail, raw secrets, and any approval. Opt-in is per feature. The default class is local-preferred. |
| MCP clients | Proposing the same typed calls the model may propose | Confirmation, policy, tokens, and the SQLCipher key. A client is untrusted. |
| Plugins | Proposing typed calls inside the capabilities a person already granted | Granting those capabilities to themselves, or approving a call. A plugin is untrusted. |
| Push relay | Waking a device | Tokens, mail content, headers, attachments, and which account a wake is for. It never sees them. |
| Host shell | Recording audio, showing the UI, and asking for confirmation | Being skipped: a confirmation that did not come from the person is a denial. |

### Provider servers

IMAP, SMTP, JMAP, Gmail, and Microsoft Graph are outside the device. Credentials used to reach them stay in the keychain. The channel being authenticated does not make a message trustworthy: phishing, HTML, and hostile headers arrive from accounts the person uses. Push frames from a server (JMAP EventSource and WebSocket) carry state strings only; the device then syncs with an ordinary authenticated request.

### Local store

`mailune-store` holds the SQLCipher database. The core applies a typed `Submission` in a transaction and queues the server operation. Rows are our copy of mail state. When a body is rendered or placed in a prompt, it is still untrusted input.

### OS keychain

`SecretStore` is the only read path for the SQLCipher key and for tokens. The core does not put those values in config, logs, events, prompts, or `Submission`. Sign-in on the contract carries an address, not a password.

### Local model

Local is the default. The router may use heuristics, the platform model through a host callback, or bundled llama.cpp. A local model may see mail whose privacy class is `local-only` or `local-preferred`, because that data stays on the device. It still only proposes a typed result. `mailune-ai` is the pure place for the privacy class and for tool permissions. Speech recognition is a local model too: the host records audio and hands over samples, and the transcript only goes into the draft being edited.

### Cloud model

Cloud runs only with the person's own key, and only for a feature they opted into (`cloud-allowed`). The key stays in the keychain and is not part of the prompt. Before a cloud call, quoted text and signatures are redacted. Each cloud call writes a ledger row: feature, provider, bytes sent, and message ids. A paid hosted tier is later and is out of scope here; [`hosted-ai.md`](hosted-ai.md) puts it behind the same router and the same classes.

### MCP clients

`mailune-mcp` forwards each tool to `Agent::request`. An external client does not become the app by connecting. Its proposals pass the same policy and scope as a model proposal, and a send is held for the person to approve in the app; no tool argument can confirm. The client does not receive tokens or the SQLCipher key. A broken client or server is skipped with a warning.

### Plugins

There is no plugin host in the tree yet. When one lands: a plugin is untrusted code, its manifest does not grant itself a capability, a new capability needs a person's approval, a broken plugin is skipped with a warning, and any call it proposes still waits for policy.

### Push relay

The relay is optional. Provider push (Gmail Pub/Sub, Graph webhooks) reaches the relay on a per-registration webhook URL, and the relay sends an empty wake to the device behind it. The device then syncs with the provider itself. The relay stores only channel-to-device routes. The device registers a route with a provider, a channel id drawn from host randomness, and its push handle; the account behind the channel stays on the device. Because the wake names no account, a device with several relay accounts syncs all of them on a wake. If a wake cannot be sent without putting content or a token in it, the wake is not sent.

## Guards

Each rule below has exactly one implementation. A change that removes one says why in its review and updates this table.

| Rule | Guard | Proof |
| --- | --- | --- |
| Encrypted mail is local-only | `mailune_ai::effective_privacy`, used by `select`, `Router::route`, and `allow_cloud` | `mailune-ai` tests: encrypted mail never selects a cloud model; smart replies on an encrypted thread fail with `CloudForbidden` |
| A cloud request needs a cloud-allowed class, and leaves quoted text and signatures behind | `mailune_ai::build_request` calls `allow_cloud`, then `redact_for_cloud` | `cloud.rs` and `redact.rs` tests |
| A feature without a policy row does not run | `Router::route` returns `NoPolicy`; an empty router fails every call | `router.rs` tests |
| Model text is never an action by itself | `guard::admit` (exact `tool:<name>` token on the allow-list) and `proposal_from_mail`, which refuses every body | `guard.rs` tests |
| Agent calls stay in scope, are previewed, audited, and undoable | `Agent::propose`, `Agent::commit`, `Agent::undo`, `AuditLine` | `agent.rs` tests |
| Send and delete wait for the person | `needs_confirmation` and `Confirmation::Confirmed`; `Agent::request` holds them for `Agent::approve` | `agent.rs` and `mailune-mcp` tests |
| Typed model results have one shape or none | `generate_json`, `rule_from_sentence` (no send, forward, or delete action exists), `admit_replies` (exactly three short suggestions, no links) | `engine.rs`, `rules.rs`, `reply.rs` tests |
| The model cannot lower a phishing risk | `phishing::combine`: facts from the MIME layer win, the model can only raise | `phishing.rs` tests |
| A model blob runs only if it is the catalogued one | `catalog::verify` (size and SHA-256), deleted on mismatch | `catalog.rs` tests |
| Mail is not logged | `Debug` of `Prompt`, `Generate`, and `MailText` hides the text; `Secret` and the `Authorization` header are redacted | `mailune-ai` tests in `lib.rs`, `engine.rs`, `summary.rs`; `mailune-protocol` tests in `host.rs`, `http.rs` |
| The relay sees no token and no mail | `mailune_push::screen` (size cap, provider shape, no token or content keys, no bearer or JWT values) and `POST /register` (three known fields only, guessable channels and token-shaped handles refused, a taken channel never re-pointed) | `screen.rs`, `relay.rs`, `client.rs` tests |
| Sources never call the OS keychain or name a remote image | `crates/mailune-cli/tests/secrets.rs` scans every crate and shell source | that test, with a planted call |
| Surfaces only forward | `crates/mailune-cli/tests/conventions.rs` (one-expression exports) and `deps.rs` (dependency direction, one owner per heavy dependency) | those tests |
| Shown HTML, links, remote content, and new senders are treated as hostile | `mailune_mime::sanitize_html`, `inspect_link`, `inspect_url`, `screen_sender`, `verify_dkim` | `mailune-mime` tests and the `mime_parse` fuzz target |

## Untrusted input

These inputs are untrusted:

- Mail content, including HTML, headers, attachments, and calendar data inside a message.
- Push frames and webhook bodies, which are screened before they wake anything.
- Model output, local or cloud, including text that claims the person already agreed, and speech transcripts.
- MCP clients, their tool results, and their resources.
- Plugins, their manifests, and their results.

Untrusted text is data. It is wrapped, sanitized, and capped before it is shown or placed next to instructions; every prompt template says the mail is data and not instructions. How mail is rendered is in [`architecture.md`](architecture.md#security) under "Rendering mail". Instructions found in mail, in a tool result, or in a plugin do not approve the next action.

The model only proposes typed results. On the contract, an action is a `Submission`. A policy outside the model approves it and fails closed:

1. The proposal is parsed as a typed call. A call that does not parse is denied.
2. Deny rules match first. A deny wins over an allow.
3. The policy engine in the core decides. The model, the tool, the plugin, and the MCP client do not check their own permission.
4. No allow, no matching rule, an unknown tool, or a missing decision is a denial. Nothing runs in that gap.
5. A call runs only after the engine returns allow. Send, delete, and forward still need the confirmation below even after an allow.
6. Agent actions are logged and can be undone.

A tool result is shown as data. It is not a new grant.

## Encrypted mail

OpenPGP and S/MIME mail is never sent to a cloud model. That covers ciphertext, decrypted plaintext, keys, and extracted text from those messages. The privacy class is forced to `local-only` by `effective_privacy`, so the router, the cloud request builder, and every feature that builds a `Prompt` inherit it. A local model may still handle that mail on device. Transport TLS (`TransportSecurity::Tls` on the envelope) is not end-to-end encryption and does not by itself force `local-only`.

## Confirmation

Send, delete, and forward always need confirmation in the app. The confirmation is a deliberate action in the Mailune UI. It is not a sentence in the model output, a line in a message, a field from an MCP client, a spoken word, or a plugin result.

The contract already says the app confirms before `Submission::Send` and `Submission::Delete` are applied. Forward follows the same rule when that action exists. There is no mode, session grant, or opt-in that skips these three. A dismissed or unanswered confirmation is a denial: the message is not sent, deleted, or forwarded. Suggestions the AI writes (replies, compose assist, dictation) go into the composer and are sent only through that confirmation.

Other actions are not given that exception here. The agent-tools layer may allow or ask for them under the fail-closed order above. It may not weaken send, delete, or forward.

## Secrets and tests

Secrets live only in the OS keychain. Config, logs, prompts, events, the op queue, and `Submission` carry handles or non-secret fields, not token values and not the SQLCipher key.

Tests never touch the real keychain or the network. They use fakes (`mailune-testkit`, injected stores, a scripted HTTP transport, routers driven in process). A test that needs a credential injects one. It does not read macOS Keychain, Android Keystore, Secret Service, or Windows Credential Manager, and it does not open a socket to a provider, a relay, or a model host.

## Known gaps

The tree does not hold every rule yet. Each gap below is tracked work or needs a decision.

- **SQLCipher only on Apple targets.** Linux, Windows, and Android open plain SQLite and refuse a key rather than pretend to encrypt (`mailune-store`), until their builds can link a crypto library.
- **Header cleaning.** Escape sequences and bidi controls in headers are not stripped yet: the shared `text-sanitize` crate is not in the tree (`docs/text-sanitize.md`, task X10). AI suggestions refuse control characters but not bidi overrides.
- **Fuzz coverage.** Three parsers have fuzz targets (`mime_parse`, `autoconfig_xml`, `search_query`). JMAP and push JSON, relay webhook bodies, IMAP responses, and iCalendar do not yet, although the architecture asks for one per parser of untrusted input.
- **Relay channel ids.** Anyone who learns a channel id can wake that device, which costs battery and a sync but reveals nothing. Rotating a channel means registering a new one; there is no unregister yet.
- **Which account a wake is for.** The wake is empty, so every relay account on the device syncs. Naming the account would put a stable per-account tag in the push that Apple or Google would see. That trade-off is for the creator.

## What this document does not decide

This file names assets, boundaries, and the rules that fail closed. It does not decide:

- Exploit procedures, attack techniques, payloads, or reproduction steps.
- The permission-rule grammar. The order (deny, then allow, then fail closed) is fixed; the syntax is later.
- How OpenPGP or S/MIME is implemented.
- The paid hosted AI tier, beyond "same router, same privacy classes".
- Sanitiser configuration or parser internals.

If a later change would drop one of these boundaries, the change has to say why in its review, and this document has to be updated in the same change.
