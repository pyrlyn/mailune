# Mailune

A local-first mail client with private AI, for macOS, iOS, Windows, Linux, Android and the web.

- **Your mail stays between you and your provider.** The app talks directly to IMAP/SMTP, JMAP, the Gmail API and Microsoft Graph. No vendor server sits in the middle.
- **AI runs on your device by default.** It uses Apple Foundation Models, Gemini Nano, Windows AI or a bundled llama.cpp. Cloud models are optional and use your own key. A ledger shows every request that leaves the device.
- **AI works on encrypted mail too.** OpenPGP and S/MIME messages are decrypted locally, and only local models ever see them.
- **Native on every platform.** One Rust core, with SwiftUI, WinUI 3, GTK4/libadwaita and Jetpack Compose interfaces.

Status: planning. Nothing is built yet. See `plan.md` and `roadmap.md`.

## License

You can use this project under **any** of the following licenses, at your choice:

1. [GNU GPLv3](LICENSE): free for open source applications on any platform, including embedded systems.
2. [Royalty-free License](LICENSE-ROYALTY-FREE.md): free for proprietary desktop, mobile, and web applications, as long as you disclose that your application uses this project. Embedded systems are not covered.
3. [Commercial license](PRICING.md): for proprietary applications, including embedded systems, without the attribution requirement.
