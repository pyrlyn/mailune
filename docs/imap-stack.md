# IMAP stack

Mailune will speak IMAP with **`imap-codec` 1.0.0** (the stable line). The client that owns the socket comes later, in `mailune-imap`, and sits on that codec. This spike does not take `async-imap`.

Checked 2026-10-08.

## What was opened

| Source | What it is |
| --- | --- |
| <https://crates.io/crates/imap-codec> | Crate page. Stable version **1.0.0**. Newest publish is **2.0.0-alpha.9** (2026-07-19). About 133,649 downloads. Repository <https://github.com/duesee/imap-codec>. |
| <https://github.com/duesee/imap-codec> | Repo, default branch `main`, not archived, last push 2026-10-05. License on the repo is Apache-2.0; the published 1.0.0 manifest is `MIT OR Apache-2.0`. |
| <https://github.com/duesee/imap-codec/blob/v1.0.0/imap-codec/Cargo.toml> | Stable manifest. Feature `ext_condstore_qresync`. Quirk `quirk_missing_text` is documented there as the Gmail `HIGHESTMODSEQ` response that omits the required text. Quirk `quirk_rectify_numbers` is documented as Dovecot sending `-1`. |
| <https://raw.githubusercontent.com/duesee/imap-codec/main/README.md> | The crate is a codec (`imap-codec` parses and serialises, `imap-types` holds the types). It implements the IMAP4rev1 formal syntax, streams (`Incomplete` rather than a truncated message), prefers zero-copy, and is fuzzed so a message it emits is one it can parse. The README points at `imap-next` for a higher-level helper. |
| <https://crates.io/crates/async-imap> | Crate page. Version **0.12.0**, published 2026-09-30. About 1,806,855 downloads. Repository <https://github.com/async-email/async-imap>. |
| <https://github.com/async-email/async-imap> | README on `main`: a client. `connect`, then `Client::login` or `authenticate`, then a `Session`. Based on `rust-imap`. Integration tests need GreenMail. License `MIT OR Apache-2.0`. |
| <https://raw.githubusercontent.com/async-email/async-imap/main/Cargo.toml> | `0.12.0` depends on `imap-proto` 0.17. Default feature `runtime-async-std`; `runtime-tokio` is optional. |

`2.0.0-alpha.9` is not the pin. The stable crate already has the extension this client needs.

## Why imap-codec

The sync model needs CONDSTORE / QRESYNC, with a UID diff when the server has neither. `imap-codec` 1.0.0 exposes that as `ext_condstore_qresync`. The same manifest already records two server bugs we will meet: Gmail's `HIGHESTMODSEQ` line, and Dovecot's `-1`.

IMAP bytes are untrusted. The architecture asks for a parser that can be fuzzed. The codec's own README says it is fuzzed never to emit a message it cannot parse, and a `Fragmentizer` keeps one bad message from desynchronising the stream.

`async-imap` is a client, not a codec. It brings `imap-proto` 0.17 and a runtime (async-std unless the tokio feature is selected) and it opens the connection itself. Mailune keeps sockets behind traits in `mailune-protocol` and gives each heavy dependency one owning crate. Wrapping `async-imap` would still leave the grammar in `imap-proto`, and the runtime choice would be the dependency's.

Downloads and a newer crates.io date favour `async-imap` (0.12.0 on 2026-09-30, versus 1.0.0 of the codec). The codec repository was still pushed on 2026-10-05. Popularity is not the parser the sync model needs.

`imap-next`, named from the codec README, was not adopted here. The task was the codec against `async-imap`. The session, IDLE, and the op queue stay in `mailune-imap` when that crate is written, on `imap-codec` 1.0.0 with `ext_condstore_qresync`.
