---
title: Mailune
tagline: A local-first mail client with private AI, for macOS, iOS, Windows, Linux, Android and the web.
repo: https://github.com/pyrlyn/mailune
install: "cargo install --locked --git https://github.com/pyrlyn/mailune mailune-cli"
install_alternatives:
  - "git clone https://github.com/pyrlyn/mailune && cd mailune && mise exec -- cargo build --workspace --locked"
version: "0.1.0"
status: "In development · no release yet"
accent: "#8FB4D9"
accent2: "#B7D0E8"
accentLight: "#335C8C"
order: 5
---

<!-- Website copy for the pyrlyn project site, copied to pyrlyn/landing as
content/projects/mailune.md; front matter follows CONTENT_CONTRACT.md in that repository.
Sources (checked 2026-10-10): README.md, AGENTS.md (decisions) and the `mailune` CLI in
crates/mailune-cli; version from the workspace Cargo.toml (no tag and no release yet, hence
`status`); accent, accent2 and accentLight are the dark-theme accent, the dark-theme
accent-bright and the seal blue in pyrlyn/brand brands/mailune/tokens.json. -->

## Overview

Mailune is a local-first mail client. One Rust core talks to the mail provider, and native apps
and a self-hosted web client sit on that core. AI runs on the device unless you opt in to a
cloud model with your own key.

Mailune is in development: there is no release and no app yet. The core crates and a
command-line tool that records accounts in a scratch directory build from source.

## Features

- **Your mail stays between you and your provider.** Mailune talks directly to IMAP and SMTP,
  JMAP, the Gmail API and Microsoft Graph. No vendor server sits in the middle.
- **AI on your device by default.** Apple Foundation Models, Gemini Nano, Windows AI or a bundled
  llama.cpp. Cloud models are optional and use your own key, and a ledger shows every request
  that leaves the device.
- **AI on encrypted mail too.** OpenPGP and S/MIME messages are decrypted locally, and only local
  models ever see them.
- **Native on every platform.** One Rust core under SwiftUI, WinUI 3, GTK4 with libadwaita and
  Jetpack Compose interfaces.
- **Self-hosted web client.** The first version ships `mailune-server`, which serves the same core
  to a browser over JSON-RPC on a WebSocket.

## Install

There is no release yet. Install the command-line tool from source (Rust 1.99):

```bash
cargo install --locked --git https://github.com/pyrlyn/mailune mailune-cli
```

Or build the whole workspace with the repository's pinned toolchain:

```bash
git clone https://github.com/pyrlyn/mailune && cd mailune && mise exec -- cargo build --workspace --locked
```

## Usage examples

Record an account in a scratch directory. Nothing is sent over the network yet.

```bash
export MAILUNE_HOME="$HOME/.mailune-scratch"
mailune account add --address ada@example.com --label Work
```

List the stored accounts, one address and label per line.

```bash
mailune account list
```

## Links

- Repository: <https://github.com/pyrlyn/mailune>
- Plan: <https://github.com/pyrlyn/mailune/blob/main/plan.md>
- License: <https://github.com/pyrlyn/mailune/blob/main/LICENSE>
