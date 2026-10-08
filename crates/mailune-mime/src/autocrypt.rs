//! Autocrypt Level 1 headers: the sender's key, and gossip keys.
//!
//! This is a header codec only. Key bytes stay opaque here; checking that
//! they are an OpenPGP key, and deciding what to store, belongs to the
//! crypto and store layers. Nothing here looks a key up over the network
//! (no WKD): a key arrives only inside mail the user already holds.
//!
//! The rules follow Autocrypt Level 1, section 2:
//! <https://autocrypt.org/level1.html#the-autocrypt-header>.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mail_parser::{Message, MessageParser, MimeHeaders};
use mailune_protocol::Address;

use crate::Error;

/// The largest key a header may carry, decoded.
///
/// A header is written by whoever sent the mail and every accepted key is
/// kept per peer, so an unbounded one would let one message fill the store.
/// Level 1 keys are minimal (one signing and one encryption subkey) and
/// fit well under this.
const MAX_KEYDATA: usize = 64 * 1024;

/// What the key owner asked for, from `prefer-encrypt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferEncrypt {
    /// `prefer-encrypt=mutual`: encrypt by default when both sides agree.
    Mutual,
    /// No value, or any value other than `mutual`.
    NoPreference,
}

/// One key announced by an `Autocrypt` or `Autocrypt-Gossip` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocryptKey {
    /// The address the key is for, as written in the header.
    pub addr: String,
    /// The owner's preference. Always [`PreferEncrypt::NoPreference`] for gossip.
    pub prefer_encrypt: PreferEncrypt,
    /// The key, base64-decoded. Not checked to be OpenPGP here.
    pub keydata: Vec<u8>,
}

/// The sender's key from the top-level `Autocrypt` header of `bytes`.
///
/// `None` unless exactly one header is valid and its `addr` is the single
/// `From` address (compared without case). Delivery reports
/// (`multipart/report`) never carry one, so a header there is ignored.
///
/// # Errors
///
/// Returns [`Error::Parse`] when the bytes are not a message.
pub fn sender_autocrypt(bytes: &[u8]) -> Result<Option<AutocryptKey>, Error> {
    let message = MessageParser::default().parse(bytes).ok_or(Error::Parse)?;
    if is_report(&message) {
        return Ok(None);
    }
    let Some(from) = single_from(&message) else {
        return Ok(None);
    };
    let mut valid = raw_values(&message, "Autocrypt").filter_map(|value| parse_header(&value));
    let (Some(key), None) = (valid.next(), valid.next()) else {
        return Ok(None);
    };
    Ok(key.addr.eq_ignore_ascii_case(&from).then_some(key))
}

/// Gossip keys from the top-level `Autocrypt-Gossip` headers of a decrypted
/// payload.
///
/// Gossip is only trusted inside the encrypted part, so `payload` must be the
/// decrypted inner message, never the outer one. A key is kept only when its
/// `addr` is one of `recipients` (the outer `To` and `Cc`): anything else is
/// the sender vouching for a stranger. The first key per address wins.
///
/// # Errors
///
/// Returns [`Error::Parse`] when the payload is not a message.
pub fn gossip_keys(payload: &[u8], recipients: &[Address]) -> Result<Vec<AutocryptKey>, Error> {
    let message = MessageParser::default()
        .parse(payload)
        .ok_or(Error::Parse)?;
    let mut keys: Vec<AutocryptKey> = Vec::new();
    for mut key in raw_values(&message, "Autocrypt-Gossip").filter_map(|value| parse_header(&value))
    {
        let known = recipients
            .iter()
            .any(|recipient| recipient.email.eq_ignore_ascii_case(&key.addr));
        let seen = keys
            .iter()
            .any(|kept| kept.addr.eq_ignore_ascii_case(&key.addr));
        if known && !seen {
            // Only the key owner may state a preference; a gossiper cannot.
            key.prefer_encrypt = PreferEncrypt::NoPreference;
            keys.push(key);
        }
    }
    Ok(keys)
}

/// Parses one header value. `None` when the header is invalid.
fn parse_header(value: &str) -> Option<AutocryptKey> {
    let mut addr = None;
    let mut prefer = None;
    let mut keydata = None;
    for attribute in value.split(';') {
        let attribute = attribute.trim();
        if attribute.is_empty() {
            continue;
        }
        let (name, value) = attribute.split_once('=')?;
        let slot = match name.trim() {
            "addr" => &mut addr,
            "prefer-encrypt" => &mut prefer,
            "keydata" => &mut keydata,
            // Underscore attributes are optional by definition; any other
            // unknown one is critical and voids the header.
            other if other.starts_with('_') => continue,
            _ => return None,
        };
        if slot.replace(value.trim()).is_some() {
            return None;
        }
    }
    let addr = addr.filter(|addr| !addr.is_empty())?;
    let keydata = decode_keydata(keydata?)?;
    let prefer_encrypt = match prefer {
        Some("mutual") => PreferEncrypt::Mutual,
        _ => PreferEncrypt::NoPreference,
    };
    Some(AutocryptKey {
        addr: addr.to_string(),
        prefer_encrypt,
        keydata,
    })
}

/// Base64 with the folding whitespace removed, within [`MAX_KEYDATA`].
fn decode_keydata(text: &str) -> Option<Vec<u8>> {
    let compact: String = text.split_ascii_whitespace().collect();
    // Four base64 characters carry three bytes, so this bounds the work
    // before any decoding happens.
    if compact.is_empty() || compact.len() / 4 * 3 > MAX_KEYDATA {
        return None;
    }
    STANDARD.decode(compact).ok()
}

/// The raw, still folded values of every root header called `name`.
fn raw_values<'m>(message: &'m Message<'_>, name: &'m str) -> impl Iterator<Item = String> + 'm {
    let raw = message.raw_message();
    message
        .headers()
        .iter()
        .filter(move |header| header.name.as_str().eq_ignore_ascii_case(name))
        .filter_map(move |header| {
            let start = usize::try_from(header.offset_start).ok()?;
            let end = usize::try_from(header.offset_end).ok()?;
            let bytes = raw.get(start..end)?;
            std::str::from_utf8(bytes).ok().map(str::to_string)
        })
}

/// The `From` address, when there is exactly one.
fn single_from(message: &Message<'_>) -> Option<String> {
    let mut emails = message
        .from()?
        .iter()
        .filter_map(|addr| addr.address.as_deref().map(str::trim));
    match (emails.next(), emails.next()) {
        (Some(email), None) if !email.is_empty() => Some(email.to_string()),
        _ => None,
    }
}

fn is_report(message: &Message<'_>) -> bool {
    message.content_type().is_some_and(|ctype| {
        ctype.ctype().eq_ignore_ascii_case("multipart")
            && ctype
                .subtype()
                .is_some_and(|sub| sub.eq_ignore_ascii_case("report"))
    })
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use mailune_protocol::Address;

    use super::{MAX_KEYDATA, PreferEncrypt, gossip_keys, sender_autocrypt};

    const KEY: &[u8] = b"\x98\x33\x04fake-openpgp-key-bytes";

    fn keydata() -> String {
        STANDARD.encode(KEY)
    }

    fn mail(headers: &str) -> Vec<u8> {
        format!("{headers}Subject: hi\r\nContent-Type: text/plain\r\n\r\nbody\r\n").into_bytes()
    }

    fn recipient(email: &str) -> Address {
        Address {
            name: None,
            email: email.into(),
        }
    }

    #[test]
    fn a_valid_header_yields_the_senders_key() {
        let bytes = mail(&format!(
            "From: Ada <Ada@Example.com>\r\nAutocrypt: addr=ada@example.com; prefer-encrypt=mutual; keydata={}\r\n",
            keydata()
        ));
        let key = sender_autocrypt(&bytes).unwrap().unwrap();
        assert_eq!(key.addr, "ada@example.com");
        assert_eq!(key.prefer_encrypt, PreferEncrypt::Mutual);
        assert_eq!(key.keydata, KEY);
    }

    #[test]
    fn folded_keydata_and_underscore_attributes_are_accepted() {
        let encoded = keydata();
        let (head, tail) = encoded.split_at(8);
        let bytes = mail(&format!(
            "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; _client=x; prefer-encrypt=nope;\r\n keydata={head}\r\n {tail}\r\n"
        ));
        let key = sender_autocrypt(&bytes).unwrap().unwrap();
        assert_eq!(key.keydata, KEY);
        assert_eq!(key.prefer_encrypt, PreferEncrypt::NoPreference);
    }

    #[test]
    fn invalid_headers_yield_nothing() {
        let k = keydata();
        let cases = [
            // addr does not match From
            format!("From: eve@example.com\r\nAutocrypt: addr=ada@example.com; keydata={k}\r\n"),
            // unknown critical attribute
            format!(
                "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; color=red; keydata={k}\r\n"
            ),
            // duplicate attribute
            format!(
                "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; addr=ada@example.com; keydata={k}\r\n"
            ),
            // missing keydata, bad base64
            "From: ada@example.com\r\nAutocrypt: addr=ada@example.com\r\n".to_string(),
            "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; keydata=***\r\n".to_string(),
            // two valid headers mean none
            format!(
                "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; keydata={k}\r\nAutocrypt: addr=ada@example.com; keydata={k}\r\n"
            ),
            // two From addresses
            format!(
                "From: ada@example.com, bob@example.com\r\nAutocrypt: addr=ada@example.com; keydata={k}\r\n"
            ),
        ];
        for case in cases {
            assert_eq!(sender_autocrypt(&mail(&case)).unwrap(), None, "{case}");
        }
    }

    #[test]
    fn an_oversized_key_is_refused() {
        let big = STANDARD.encode(vec![0u8; MAX_KEYDATA + 3]);
        let bytes = mail(&format!(
            "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; keydata={big}\r\n"
        ));
        assert_eq!(sender_autocrypt(&bytes).unwrap(), None);
    }

    #[test]
    fn a_delivery_report_carries_no_key() {
        let bytes = format!(
            "From: ada@example.com\r\nAutocrypt: addr=ada@example.com; keydata={}\r\nContent-Type: multipart/report; report-type=delivery-status; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nx\r\n--b--\r\n",
            keydata()
        );
        assert_eq!(sender_autocrypt(bytes.as_bytes()).unwrap(), None);
    }

    #[test]
    fn gossip_is_kept_for_recipients_only_and_states_no_preference() {
        let k = keydata();
        let payload = mail(&format!(
            "Autocrypt-Gossip: addr=bob@example.com; prefer-encrypt=mutual; keydata={k}\r\n\
             Autocrypt-Gossip: addr=stranger@example.com; keydata={k}\r\n\
             Autocrypt-Gossip: addr=BOB@example.com; keydata=AAAA\r\n\
             Autocrypt-Gossip: addr=cy@example.com; keydata={k}\r\n"
        ));
        let keys = gossip_keys(
            &payload,
            &[recipient("bob@example.com"), recipient("cy@example.com")],
        )
        .unwrap();
        let addrs: Vec<_> = keys.iter().map(|key| key.addr.as_str()).collect();
        assert_eq!(addrs, ["bob@example.com", "cy@example.com"]);
        assert_eq!(keys[0].keydata, KEY);
        assert!(
            keys.iter()
                .all(|key| key.prefer_encrypt == PreferEncrypt::NoPreference)
        );
    }

    #[test]
    fn unparseable_bytes_are_a_parse_error() {
        assert!(sender_autocrypt(b"").is_err());
        assert!(gossip_keys(b"", &[]).is_err());
    }
}
