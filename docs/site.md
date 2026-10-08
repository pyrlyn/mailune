# Mailune

Mailune is a local-first mail client. One Rust core talks to the mail provider. Native apps and a self-hosted web client sit on that core. AI runs on the device unless a person opts in to a cloud model with their own key.

Mail stays between the person and their provider: IMAP, SMTP, JMAP, the Gmail API, and Microsoft Graph. There is no Mailune server in that path. Encrypted mail is decrypted on the device, and only a local model may see it.

## License

Use Mailune under any one of these, at your choice:

- [GNU GPLv3](../LICENSE) (`GPL-3.0-or-later`)
- [Royalty-free license](../LICENSE-ROYALTY-FREE.md) for proprietary desktop, mobile, and web applications that disclose their use of Mailune
- [Commercial license](../PRICING.md), on request, including embedded systems

The crate license field is `GPL-3.0-or-later OR LicenseRef-Mailune-Royalty-Free`.

## Brand

`brands/mailune` is not added in the brand repository. That repository keeps an explicit brand list in `build.mjs`, and each brand needs tokens, logos, package exports, and a regenerated `dist/`. An incomplete folder would fail those checks. The brand entry is a follow-up.
