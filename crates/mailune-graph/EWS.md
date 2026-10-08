# On-premises EWS

Checked 2026-10-08.

Exchange Online stays on Microsoft Graph in this crate. On-premises Exchange
is a separate SOAP API. Thunderbird publishes the types for that API as the
`ews` crate.

## What `ews` is

Primary sources:

- Repository README, crate `ews` 0.1.2: <https://github.com/thunderbird/ews-rs>
- Crate manifest `ews/Cargo.toml` in that repository: license `MPL-2.0`,
  dependencies `quick-xml`, `serde`, `xml_struct`, `time`, `thiserror`, and
  `ews_proc_macros`. No HTTP client.
- `ews/src/lib.rs`: the public error is XML serialize, deserialize, I/O, and
  SOAP fault. The file header is the standard MPL notice. It does not carry
  the "Incompatible With Secondary Licenses" notice.
- Registry metadata for 0.1.2: license `MPL-2.0`, repository the same GitHub
  URL. <https://crates.io/crates/ews>

The crate serializes a `GetFolder` operation to a SOAP document and parses a
SOAP response back into typed folders. It does not open a socket.

## Reuse

The crate is reusable. Mailune depends on it and posts the bytes through an
injected transport. Tests use a scripted response. MPL source stays in the
`ews` crate. It is not copied into a Mailune file.

Mozilla Public License 2.0 is compatible with GPL-3.0 when the secondary
license incompatibility notice is absent. Text: <https://www.mozilla.org/MPL/2.0/>
