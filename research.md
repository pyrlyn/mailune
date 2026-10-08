# Research: mail clients and AI features

Checked 2026-10-08 unless a line says otherwise. Every fact carries its primary source. **unverified** marks a fact backed only by secondary sources or search snippets. Superhuman's help centre is behind a bot challenge, so facts taken from its snippets are marked.

## Clients

### Gmail + Gemini (web, iOS, Android)

**AI features**
- Gemini 3 runs in the cloud.
- Free tier:
  - AI Overviews (thread summaries).
  - Help me write.
  - Suggested Replies that match the user's style. On free personal accounts these are US-only.
- Paid tier: Proofread (web), Q&A in search, AI Inbox (beta, US/English), Gmail Live (beta).
- Workspace only: Help me schedule.

**Limits of AI Inbox**
- Reads only the Primary tab.
- Skips attachments and encrypted mail.
- First results can take up to 7 days.

**Privacy**
- Workspace and AI-plan content is not used for training.
- AI needs the "smart features" settings turned on.

**Protocols and encryption**
- Third-party fetch is shrinking: Gmailify has been removed, and POP fetch closed to new users after Q1 2026.
- No PGP. S/MIME and client-side encryption are Workspace only.

**Sources**
- https://support.google.com/mail/answer/16831098
- https://blog.google/products-and-platforms/products/gmail/gmail-is-entering-the-gemini-era/ (2026-01-08)
- https://support.google.com/mail/answer/16845247
- https://support.google.com/mail/answer/14615114
- https://support.google.com/mail/answer/16604719

### Outlook + Copilot (Windows, macOS, web, iOS, Android)

**AI features**
- Prioritize my inbox, with a stated reason per message.
- Thread and attachment summaries.
- Drafting with organisational context, plus tone coaching.
- Chat over mail and calendar, scheduling, natural-language rules.
- Agentic background triage through Frontier: **unverified**.

**Limits**
- Exchange Online primary mailbox only.
- No shared or delegate mailboxes, and no S/MIME, DKE or IRM mail.

**Privacy**
- Runs in the cloud on Azure OpenAI.
- Anthropic and OpenAI can be enabled by admins as subprocessors.
- No training on prompts.

**Price**
- Copilot Business is $21 per user per month; Microsoft 365 Copilot is $30.
- Consumer plans include AI credits, and these also apply to connected Gmail, Yahoo and iCloud accounts.

**Protocols**
- New Outlook does not support on-premises Exchange.
- EWS on Exchange Online is blocked from 2026-10-01 and shuts down on 2027-04-01.

**Sources**
- https://support.microsoft.com/en-US/Outlook/frequently-asked-questions-about-copilot-in-outlook (2026-08)
- https://learn.microsoft.com/en-us/copilot/microsoft-365/microsoft-365-copilot-privacy (2026-09-30)
- https://learn.microsoft.com/en-us/microsoft-365-apps/outlook/get-started/supported-account-types
- https://learn.microsoft.com/en-us/exchange/clients-and-mobile-in-exchange-online/deprecation-of-ews-exchange-online

### Apple Mail + Apple Intelligence (macOS, iOS, iPadOS)

**AI features**
- Summary previews, Priority Messages, Smart Reply, Writing Tools.
- Categories: Primary, Transactions, Updates, Promotions.
- The 27 releases (2026-09) add rebuilt search, per-recipient tone, and Siri AI that acts on mail.

**Where it runs and price**
- On-device, plus Private Cloud Compute, which is stateless and keeps a public transparency log.
- Free, with limits.
- Siri AI is an English beta, unavailable in the EU on iOS and in China.

**Protocols**
- IMAP, POP, SMTP. Exchange via ActiveSync (iOS) and EWS (Mac).
- The move to Graph is promised for a "future macOS 27 update" (search snippet).

**Sources**
- https://support.apple.com/guide/iphone/iph9ae667055/ios
- https://www.apple.com/newsroom/2026/09/siri-ai-a-profoundly-more-capable-and-personal-assistant-is-here/
- https://security.apple.com/blog/private-cloud-compute/
- https://support.apple.com/guide/deployment/integrate-with-microsoft-exchange-dep158966b23/web

### Thunderbird / Thundermail (Windows, macOS, Linux, Android; iOS in development)

**AI features**
- None in the core app.
- Thunderbird Assist (local first, with a confidential fallback) is still in R&D.
- AI today comes from add-ons such as ThunderAI. Thunderbolt is an early MPL-2.0 AI client.

**Protocols and encryption**
- IMAP, POP, SMTP, CalDAV, CardDAV.
- EWS: native support since version 145 is **unverified**.
- Graph for Microsoft 365 since version 154 (2026-08-18), mail only.
- Built-in OpenPGP and S/MIME.

**Mobile**
- The iOS app is native SwiftUI, IMAP first and JMAP later. There is no release date.

**Server**
- Thundermail runs on Stalwart (JMAP, IMAP, SMTP) and is in beta at $6 per month.

**Sources**
- https://www.thunderbird.net/en-US/thunderbird/154.0/releasenotes/
- https://blog.thunderbird.net/2026/09/thunderbird-desktop-new-protocol-support-microsoft-graph-api/
- https://blog.thunderbird.net/2026/10/thunderbird-for-ios-a-first-look-at-the-native-iphone-email-app-in-development/
- https://tb.pro/en-US/
- https://github.com/thunderbird/thunderbolt

### Spark (Mac, Windows, iOS, Android)

**AI features**
- Compose, quick replies, proofread, tone, translation, summaries.
- The assistant keeps a private index on the device and sends the few relevant emails to the provider. The provider keeps them for up to 30 days.
- Spark CLI (2026-05) lets outside agents work through the local store.

**Privacy and providers**
- Providers: Azure OpenAI, OpenAI, Anthropic, Vertex.
- Spark's servers hold OAuth tokens and keep recent mail for 4 hours.

**Price**
- Plus is $10 per month and Pro is $20.

**Sources**
- https://sparkmailapp.com/features/ai-assistant
- https://sparkmailapp.com/blog/introducing-spark-cli
- https://sparkmailapp.com/legal/privacy-app (2026-09-10)
- https://sparkmailapp.com/pricing

### Superhuman Mail (Mac, Windows, web, iOS, Android)

**AI features**
- Auto Labels, Auto Archive, Auto Reminders, Auto Drafts.
- Write with AI in each recipient's tone.
- Ask AI with citations.
- A Mail MCP server (2026-04-29) that works from a server-side index.

**Providers and privacy**
- Subprocessors: Anthropic, Azure, Baseten, OpenAI.
- Retention terms conflict between versions of the help centre: **unverified**.

**Price and protocols**
- From $33–40 per user per month.
- Gmail and Outlook only.

**Sources**
- https://superhuman.com/products/whats-new
- https://blog.superhuman.com/superhuman-mail-mcp/
- https://www.superhuman.com/subprocessors
- https://superhuman.com/plans

### Shortwave (web, Mac, Windows, iOS, Android)

**AI features**
- An assistant that searches, organises, schedules and writes.
- Plain-English filters.
- MCP client (beta).

**Price and models**
- Business $30 per month (Claude Sonnet 4.6), Premier $45, Max $120 (Claude Opus 4.6).

**Protocols**
- Requires Google sign-in. Microsoft 365 is not supported.

**Sources**
- https://www.shortwave.com/pricing/
- https://www.shortwave.com/security/
- https://www.shortwave.com/docs/how-tos/microsoft-outlook-exchange-other-sign-in-support/

### Canary Mail and Dove

**AI features**
- Copilot drafts, summaries and chat in the cloud.
- Prioritisation models are trained on the device.
- Semantic search is still "working on" (2026-04).

**Privacy and price**
- Push mode stores credentials on Canary's server.
- $36 or $100 per year.

**Dove**
- A new app with Focus, Feed and Noise views and a daily brief. Its terms were not found.

**Sources**
- https://canarymail.io/privacy-policy
- https://canarymail.io/pricing
- https://canarymail.io/blog/semantic-search-in-email
- https://dove.email/

### HEY (all platforms, incl. Linux Snap)

**AI features**
- No built-in model. Users bring their own agent through an MIT-licensed CLI and an MCP server.

**UX**
- The Screener for first-time senders.
- Imbox, The Feed, Paper Trail.
- Reply Later, Set Aside.

**Protocols and price**
- No IMAP. $99 per year.

**Sources**
- https://www.hey.com/agents/
- https://github.com/basecamp/hey-cli
- https://www.hey.com/features/

### Proton Mail (Scribe, Lumo)

**AI features**
- Scribe is a writing assistant only.
- It runs in server mode or in local mode (3.8 GB, English, desktop only).
- Lumo is a separate chatbot with no Mail integration.

**Protocols**
- Bridge provides IMAP and SMTP on paid plans.
- The Linux app is in beta.
- The new mobile apps share a Rust core, but that core is not public.

**Sources**
- https://proton.me/support/proton-scribe-writing-assistant
- https://proton.me/support/lumo-privacy
- https://proton.me/blog/2026-spring-summer-roadmaps
- https://proton.me/mail/bridge
- https://proton.me/blog/new-mail-apps

### Fastmail

**AI**
- None in the product.
- An official MCP server (2026-04-22) with read, write and send scopes.

**JMAP**
- Fastmail co-authored the JMAP RFCs.
- Its public API covers Mail, Contacts and MaskedEmail.
- Calendars are still on CalDAV.

**Sources**
- https://fastmail.com/blog/an-mcp-server-for-fastmail
- https://www.fastmail.com/dev/
- https://www.rfc-editor.org/rfc/rfc8620.html

### Mimestream (macOS; iOS beta)

**AI**
- Apple Writing Tools only.

**Core**
- A native Gmail API client.
- Private Push relay (1.10, 2026-07-14) that "cannot read your email". This is the pattern for Mailune's relay.

**Price**
- $4.99 per month or $49.99 per year.

**Sources**
- https://mimestream.com/releases
- https://mimestream.com/blog/1.10-released
- https://mimestream.com/pricing

### Others

- **eM Client.** Its AI is ChatGPT-based. It supports PGP and S/MIME, and Graph is planned for v11.
- **Missive.** Bring-your-own-key AI or credits, with MCP in both directions. $24 per user per month.
- **Tuta.** No AI, by design.
- **Mailspring.** No LLM features. A native C++ sync engine, MIT-licensed.
- **Linux.** Betterbird, Geary (stagnant since 46.0) and Evolution have no AI. Partial Graph support in Evolution is **unverified**.
- **Notion Mail.** Shut down on 2026-09-22.
- **Zero (0.email).** Last commit on 2025-08-31.
- **Cora, Inbox Zero.** Gmail-only assistants.

## JMAP status

- **Published RFCs:**
  - Core and Mail: 8620 Core, 8621 Mail.
  - Transport and blobs: 8887 WebSocket, 9404 Blob.
  - Messages and Sieve: 9007 MDN, 9219 S/MIME verify, 9661 Sieve.
  - Accounts and sharing: 9425 Quotas, 9670 Sharing.
  - Contacts: 9553 JSContact, 9610 Contacts.
  - Push: 9749 Web Push.
- **Calendars** is in the RFC Editor queue.
- **Servers:** Fastmail, Stalwart 0.16.25, Cyrus 3.12, Apache James (partial).

**Sources**
- https://datatracker.ietf.org/wg/jmap/documents/
- https://jmap.io/software/index.html
- https://github.com/stalwartlabs/stalwart

## Platform AI

- **Apple.** Foundation Models framework on OS 26 and later.
- **Android.** ML Kit GenAI with Gemini Nano, in beta and only on allow-listed devices.
- **Windows.** The Windows AI APIs are a Limited Access Feature and need a Copilot+ NPU. Phi Silica is being replaced by "Aion Instruct" (Insider 2026-11, retail 2027-01). Foundry Local 2.1.0 offers an OpenAI-compatible endpoint.
- **Linux.** No OS model, so Mailune bundles llama.cpp (through runa-engine) or talks to a local OpenAI-compatible endpoint.

## Gaps Mailune targets

1. Private AI on every platform: local models first, cloud only with the user's own key, and a ledger of every request that leaves the device.
2. No vendor server between the device and the mail provider.
3. AI over encrypted mail, using local models only.
4. The same AI features over IMAP, JMAP, the Gmail API and Graph.
5. The EWS-to-Graph migration window (2026-10 to 2027-04).
6. Linux and true six-platform coverage, with native widgets.
7. A JMAP-first client.
8. Agents under the user's control: a scoped local MCP server with an audit log and undo.
9. Honest pricing and full export of mail and settings, so users are never locked in (the Notion Mail lesson).
