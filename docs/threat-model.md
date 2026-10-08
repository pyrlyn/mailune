# Threat model

This document records the assets, trust boundaries, and fail-closed rules for the Mailune core, the AI path, and MCP. It is a draft for the creator to review. Later work implements these rules; this file only states them.

The shape of the rules matches the rest of the workspace. Policy is deterministic code outside the model and fails closed. One engine allows, denies, or escalates a call, and the caller does not approve itself. Deny is decided before allow. A missing, malformed, or unknown decision is a denial. A broken MCP server or plugin is warned about and skipped; that skip is not an approval.

## Assets

| Asset | What it is | Where it may live |
| --- | --- | --- |
| Mail content | Headers, bodies, attachments, drafts, and search text | Provider servers and the encrypted local store. Treated as data everywhere else. |
| Tokens | OAuth and other provider credentials | OS keychain only, read through the `SecretStore` host callback. |
| SQLCipher key | Key that opens the local database | OS keychain only. It is not stored in the database file. |
| Account config | Hosts, identity, privacy class, and per-feature opt-in | Local config. It does not hold tokens, passwords, or the SQLCipher key. |

Mail content stays an asset after it is stored. Owning a copy does not make the bytes trusted instructions.

## Trust boundaries

The device talks to providers directly. The encrypted local database is the source of truth for the UI. Servers are the source of truth for mail state. Surfaces (`mailune-ffi`, `mailune-capi`, `mailune-rpc`, `mailune-wasm`, `mailune-cli`, `mailune-mcp`, `mailune-server`) only forward calls. They do not decide policy.

| Boundary | Trusted for | Not trusted for |
| --- | --- | --- |
| Provider servers | Account mail state, after an authenticated session | Message content as instructions. Bodies, HTML, and headers are data. |
| Local store | Our encrypted records (SQLCipher, FTS5, blobs) as the UI's copy | Turning stored mail into an instruction or a secret. The key is not in the file. |
| OS keychain | Holding the SQLCipher key and provider tokens. Android uses Keystore through the host callback. | Deciding which tool runs, or confirming send, delete, or forward. |
| Local model | Proposing typed tool calls on device (platform model or bundled llama.cpp). This is the default. | Approving those calls, or confirming send, delete, or forward. Its text is untrusted. |
| Cloud model | A bring-your-own-key provider, and only when that feature is opted in | Encrypted mail, raw secrets, and any approval. Opt-in is per feature. The default class is local-preferred. |
| MCP clients | Proposing the same typed calls the model may propose | Confirmation, policy, tokens, and the SQLCipher key. A client is untrusted. |
| Plugins | Proposing typed calls inside the capabilities a person already granted | Granting those capabilities to themselves, or approving a call. A plugin is untrusted. |
| Push relay | Waking a device | Tokens, mail content, headers, and attachments. It never sees them. |

### Provider servers

IMAP, SMTP, JMAP, Gmail, and Microsoft Graph are outside the device. Credentials used to reach them stay in the keychain. The channel being authenticated does not make a message trustworthy: phishing, HTML, and hostile headers arrive from accounts the person uses.

### Local store

`mailune-store` holds the SQLCipher database. The core applies a typed `Submission` in a transaction and queues the server operation. Rows are our copy of mail state. When a body is rendered or placed in a prompt, it is still untrusted input.

### OS keychain

`SecretStore` is the only read path for the SQLCipher key and for tokens. The core does not put those values in config, logs, events, prompts, or `Submission`. Sign-in on the contract carries an address, not a password.

### Local model

Local is the default. The router may use heuristics, the platform model through a host callback, or bundled llama.cpp. A local model may see mail whose privacy class is `local-only` or `local-preferred`, because that data stays on the device. It still only proposes a typed call. `mailune-ai` is the pure place for the privacy class and for tool permissions.

### Cloud model

Cloud runs only with the person's own key, and only for a feature they opted into (`cloud-allowed`). The key stays in the keychain and is not part of the prompt. Before a cloud call, quoted text and signatures are redacted. Each cloud call writes a ledger row: feature, provider, bytes sent, and message ids. A paid hosted tier is later and is out of scope here; it must sit behind the same router and the same classes.

### MCP clients

`mailune-mcp` forwards. An external client does not become the app by connecting. Its proposals pass the same policy and the same in-app confirmation as a model proposal. The client does not receive tokens or the SQLCipher key. A broken client or server is skipped with a warning.

The later MCP server defaults to read-only, with folder or label scopes the person grants explicitly, and send still needs the in-app confirmation. This document states that boundary. It does not design the server.

### Plugins

A plugin is untrusted code. Its manifest does not grant itself a capability. A new capability needs a person's approval. A broken plugin is skipped with a warning. Any call it proposes still waits for policy.

### Push relay

The relay is optional. Provider push (Gmail Pub/Sub, Graph webhooks) reaches the relay, which sends an empty device wake. The device then syncs with the provider itself. The relay's payload is a wake, not mail and not a credential. If a wake cannot be sent without putting content or a token in it, the wake is not sent.

## Untrusted input

These inputs are untrusted:

- Mail content, including HTML, headers, attachments, and calendar data inside a message.
- Model output, local or cloud, including text that claims the person already agreed.
- MCP clients, their tool results, and their resources.
- Plugins, their manifests, and their results.

Untrusted text is data. It is wrapped, sanitized, and capped before it is shown or placed next to instructions. Headers are cleaned of escape sequences and bidi controls. HTML is sanitized and then shown in a sandboxed view with no JavaScript and no remote content by default. Links show their real target. Instructions found in mail, in a tool result, or in a plugin do not approve the next action.

The model only proposes typed tool calls. On the contract, that shape is `Submission`. A policy outside the model approves them and fails closed:

1. The proposal is parsed as a typed call. A call that does not parse is denied.
2. Deny rules match first. A deny wins over an allow.
3. The policy engine in the core decides. The model, the tool, the plugin, and the MCP client do not check their own permission.
4. No allow, no matching rule, an unknown tool, or a missing decision is a denial. Nothing runs in that gap.
5. A call runs only after the engine returns allow. Send, delete, and forward still need the confirmation below even after an allow.
6. Agent actions are logged and can be undone.

A tool result is shown as data. It is not a new grant.

## Encrypted mail

OpenPGP and S/MIME mail is never sent to a cloud model. That covers ciphertext, decrypted plaintext, keys, and extracted text from those messages. The privacy class is forced to `local-only`. A local model may still handle that mail on device. Transport TLS (`TransportSecurity::Tls` on the envelope) is not end-to-end encryption and does not by itself force `local-only`.

End-to-end crypto is a later crate. The rule above is already binding on the router, the cloud adapter, and any prompt builder.

## Confirmation

Send, delete, and forward always need confirmation in the app. The confirmation is a deliberate action in the Mailune UI. It is not a sentence in the model output, a line in a message, a field from an MCP client, or a plugin result.

The contract already says the app confirms before `Submission::Send` and `Submission::Delete` are applied. Forward follows the same rule when that action exists. There is no mode, session grant, or opt-in that skips these three. A dismissed or unanswered confirmation is a denial: the message is not sent, deleted, or forwarded.

Other actions are not given that exception here. The later agent-tools layer may allow or ask for them under the fail-closed order above. It may not weaken send, delete, or forward.

## Secrets and tests

Secrets live only in the OS keychain. Config, logs, prompts, events, the op queue, and `Submission` carry handles or non-secret fields, not token values and not the SQLCipher key.

Tests never touch the real keychain or the network. They use fakes (`mailune-testkit` and injected stores). A test that needs a credential injects one. It does not read macOS Keychain, Android Keystore, Secret Service, or Windows Credential Manager, and it does not open a socket to a provider or a model host.

## What this document does not decide

This file names assets, boundaries, and the rules that fail closed. It does not decide:

- Exploit procedures, attack techniques, payloads, or reproduction steps.
- The permission-rule grammar. The order (deny, then allow, then fail closed) is fixed; the syntax is later.
- How OpenPGP or S/MIME is implemented.
- The push relay's wire format, beyond "a wake, with no token and no mail".
- The paid hosted AI tier, beyond "same router, same privacy classes".
- Sanitiser configuration, fuzz harnesses, or parser internals. Architecture already requires a fuzz target for every parser of untrusted input.

If a later change would drop one of these boundaries, the change has to say why in its review, and this document has to be updated in the same change.
