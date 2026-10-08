//! Browser surface. Each export forwards one call into the shared crates
//! and hands JSON back, so the web client parses, threads and searches with
//! the same code as the native shells.
//!
//! Build the module with
//! `cargo rustc -p mailune-wasm --target wasm32-unknown-unknown --crate-type cdylib`.

mod json;

use wasm_bindgen::prelude::wasm_bindgen;

/// Reads an `Envelope` from JSON and writes it back in canonical form.
///
/// # Errors
///
/// The JSON error when the text is not an envelope.
#[wasm_bindgen(js_name = normalizeEnvelope)]
pub fn normalize_envelope(text: &str) -> Result<String, String> {
    json::envelope(text)
}

/// Parses an RFC 5322 message into headers and a part list, as JSON.
///
/// # Errors
///
/// The parser's error when the bytes are not a message.
#[wasm_bindgen(js_name = parseMime)]
pub fn parse_mime(raw: &[u8]) -> Result<String, String> {
    mailune_mime::parse(raw)
        .map(|message| json::mime(&message))
        .map_err(|error| error.to_string())
}

/// Threads messages given as JSON (`messageId`, `inReplyTo`, `references`,
/// `subject`, `providerThread`) and returns the thread trees.
///
/// # Errors
///
/// The JSON error when the input is not a list of messages.
#[wasm_bindgen(js_name = threadMessages)]
pub fn thread_messages(text: &str) -> Result<String, String> {
    json::threadables(text).map(|messages| json::threads(&mailune_core::thread_messages(&messages)))
}

/// Parses a search query into its terms, as JSON.
///
/// # Errors
///
/// The parser's error for a bad token.
#[wasm_bindgen(js_name = parseQuery)]
pub fn parse_query(text: &str) -> Result<String, String> {
    mailune_core::parse_query(text)
        .map(|query| json::terms(&query.terms))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    //! Native parity: each export must say what the native call says.

    use mailune_core::{Container, Term, Threadable};
    use mailune_protocol::{Address, Envelope, Flags, MessageId, ThreadId, TransportSecurity};
    use serde_json::Value;

    use super::{normalize_envelope, parse_mime, parse_query, thread_messages};

    const RAW: &[u8] = b"From: \"Ada Lovelace\" <ada@example.com>\r\nTo: bob@example.com\r\nSubject: Dock\r\nMessage-ID: <m2@example.com>\r\nIn-Reply-To: <m1@example.com>\r\nReferences: <m1@example.com>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nSee you at the dock.\r\n--b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=map.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERg==\r\n--b--\r\n";

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn protocol_types_round_trip_unchanged() {
        let envelope = Envelope {
            id: MessageId::new("m1"),
            thread: ThreadId::new("t1"),
            from: Address {
                name: Some("Ada".into()),
                email: "ada@example.com".into(),
            },
            to: Vec::new(),
            cc: Vec::new(),
            subject: "Dock".into(),
            stamp: "2026-03-01T12:00:00Z".into(),
            snippet: String::new(),
            flags: Flags {
                seen: true,
                flagged: false,
                draft: false,
                answered: false,
                deleted: false,
                keywords: vec!["work".into()],
            },
            attachment_count: 0,
            transport: TransportSecurity::Tls,
        };
        let native = serde_json::to_string(&envelope).unwrap();
        let back: Envelope = serde_json::from_str(&normalize_envelope(&native).unwrap()).unwrap();
        assert_eq!(back, envelope);
        assert!(normalize_envelope("{}").is_err());
    }

    #[test]
    fn mime_parse_matches_the_native_parser() {
        let native = mailune_mime::parse(RAW).unwrap();
        let web = parsed(&parse_mime(RAW).unwrap());
        assert_eq!(web["subject"], native.subject.as_str());
        assert_eq!(web["from"][0]["email"], native.from[0].email.as_str());
        assert_eq!(web["from"][0]["name"], "Ada Lovelace");
        assert_eq!(web["messageId"], native.message_id.as_deref().unwrap());
        assert_eq!(web["references"][0], native.references[0].as_str());
        assert_eq!(web["parts"].as_array().unwrap().len(), native.parts.len());
        let filenames: Vec<Option<&str>> =
            native.parts.iter().map(|p| p.filename.as_deref()).collect();
        assert!(filenames.contains(&Some("map.pdf")));
        assert!(
            web["parts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["role"] == "attachment" && p["filename"] == "map.pdf")
        );
        assert_eq!(
            parse_mime(b"\xff\xfe").is_err(),
            mailune_mime::parse(b"\xff\xfe").is_err()
        );
    }

    fn walk(node: &Container, web: &Value) {
        assert_eq!(web["messageId"].as_str(), node.message_id.as_deref());
        let children = web["children"].as_array().unwrap();
        assert_eq!(children.len(), node.children.len());
        for (native, web) in node.children.iter().zip(children) {
            walk(native, web);
        }
    }

    #[test]
    fn threading_matches_the_native_engine() {
        let input = r#"[
            {"messageId":"m1@x","subject":"Dock"},
            {"messageId":"m2@x","inReplyTo":"m1@x","references":["m1@x"],"subject":"Re: Dock"},
            {"messageId":"m3@x","references":["m1@x","m2@x"],"subject":"Re: Dock"},
            {"messageId":"m4@x","subject":"Invoice"}
        ]"#;
        let native = mailune_core::thread_messages(&[
            threadable("m1@x", None, &[], "Dock"),
            threadable("m2@x", Some("m1@x"), &["m1@x"], "Re: Dock"),
            threadable("m3@x", None, &["m1@x", "m2@x"], "Re: Dock"),
            threadable("m4@x", None, &[], "Invoice"),
        ]);
        let web = parsed(&thread_messages(input).unwrap());
        let roots = web.as_array().unwrap();
        assert_eq!(roots.len(), native.len());
        for (native, web) in native.iter().zip(roots) {
            walk(native, web);
        }
        assert!(thread_messages("[{}]").is_err());
    }

    fn threadable(id: &str, reply: Option<&str>, refs: &[&str], subject: &str) -> Threadable {
        Threadable {
            message_id: id.into(),
            in_reply_to: reply.map(str::to_string),
            references: refs.iter().map(|r| r.to_string()).collect(),
            subject: subject.into(),
            provider_thread: None,
        }
    }

    #[test]
    fn the_query_parser_matches_the_native_parser() {
        let text =
            r#"from:ada has:attachment before:2026-03-01 is:unread label:work "dock meeting""#;
        let native = mailune_core::parse_query(text).unwrap();
        let web = parsed(&parse_query(text).unwrap());
        let web = web.as_array().unwrap();
        assert_eq!(web.len(), native.terms.len());
        for (term, web) in native.terms.iter().zip(web) {
            let agrees = match term {
                Term::From(value) => web["from"] == value.as_str(),
                Term::To(value) => web["to"] == value.as_str(),
                Term::HasAttachment => web["hasAttachment"] == true,
                Term::Before(date) => {
                    web["before"] == format!("{}-{:02}-{:02}", date.year, date.month, date.day)
                }
                Term::Unread => web["unread"] == true,
                Term::Label(value) => web["label"] == value.as_str(),
                Term::Text(value) => web["text"] == value.as_str(),
            };
            assert!(agrees, "{term:?} vs {web}");
        }
        assert_eq!(
            parse_query("nope:x").unwrap_err(),
            mailune_core::parse_query("nope:x").unwrap_err().to_string()
        );
    }
}
