//! Autocrypt Level 1 header codec.
//!
//! The bytes are the message the caller already holds. An unknown critical
//! attribute, a repeated attribute, or a `type` other than `1` drops that
//! header. Gossip never carries `prefer-encrypt`. Nothing here looks up WKD
//! or opens a socket.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mail_parser::{HeaderValue, MessageParser};

/// Whether the sender asked for encryption by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferEncrypt {
    /// `prefer-encrypt=mutual`.
    Mutual,
    /// Absent, or `prefer-encrypt=nopreference`.
    NoPreference,
}

/// One `Autocrypt` header that parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocryptHeader {
    /// The `addr` attribute.
    pub addr: String,
    /// Decoded `keydata`. The bytes are not checked as an OpenPGP key.
    pub keydata: Vec<u8>,
    /// `prefer-encrypt`, or [`PreferEncrypt::NoPreference`] when the attribute is absent.
    pub prefer_encrypt: PreferEncrypt,
}

/// One `Autocrypt-Gossip` key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipKey {
    /// The `addr` attribute.
    pub addr: String,
    /// Decoded `keydata`.
    pub keydata: Vec<u8>,
}

/// Parse one `Autocrypt` header value, or a full header line.
pub fn parse_autocrypt(header: &str) -> Option<AutocryptHeader> {
    let parsed = attributes(header, false)?;
    Some(AutocryptHeader {
        addr: parsed.addr,
        keydata: parsed.keydata,
        prefer_encrypt: parsed.prefer_encrypt.unwrap_or(PreferEncrypt::NoPreference),
    })
}

/// The single `Autocrypt` header that matches the only `From` address.
///
/// Two matching headers, or more than one `From` address, means the message
/// has no Autocrypt header. A header for a different address is ignored.
pub fn autocrypt_header(message: &[u8]) -> Option<AutocryptHeader> {
    let message = MessageParser::default().parse(message)?;
    let from = single_from(&message)?;
    let mut found = None;
    for header in message.headers() {
        if !header.name().eq_ignore_ascii_case("autocrypt") {
            continue;
        }
        let Some(text) = value_text(header.value()) else {
            continue;
        };
        let Some(parsed) = parse_autocrypt(&text) else {
            continue;
        };
        if !parsed.addr.eq_ignore_ascii_case(&from) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(parsed);
    }
    found
}

/// `Autocrypt-Gossip` keys in header order. A header that does not parse is skipped.
pub fn gossip_keys(message: &[u8]) -> Vec<GossipKey> {
    let Some(message) = MessageParser::default().parse(message) else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    for header in message.headers() {
        if !header.name().eq_ignore_ascii_case("autocrypt-gossip") {
            continue;
        }
        let Some(text) = value_text(header.value()) else {
            continue;
        };
        if let Some(parsed) = attributes(&text, true) {
            keys.push(GossipKey {
                addr: parsed.addr,
                keydata: parsed.keydata,
            });
        }
    }
    keys
}

struct Parsed {
    addr: String,
    keydata: Vec<u8>,
    prefer_encrypt: Option<PreferEncrypt>,
}

fn attributes(header: &str, gossip: bool) -> Option<Parsed> {
    let unfolded = header.replace("\r\n", "").replace('\n', "");
    let value = strip_header_name(unfolded.trim());
    let mut addr = None;
    let mut keydata = None;
    let mut prefer_encrypt = None;
    let mut seen = Vec::new();
    for part in value.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, raw) = part.split_once('=')?;
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty() || seen.iter().any(|seen: &String| seen == &key) {
            return None;
        }
        seen.push(key.clone());
        match key.as_str() {
            "addr" => addr = Some(addr_spec(raw)?),
            "keydata" => keydata = Some(decode_keydata(raw)?),
            "prefer-encrypt" => {
                if gossip {
                    return None;
                }
                prefer_encrypt = Some(prefer(raw)?);
            }
            "type" => {
                if raw.trim() != "1" {
                    return None;
                }
            }
            other if other.starts_with('_') => {}
            _ => return None,
        }
    }
    Some(Parsed {
        addr: addr?,
        keydata: keydata?,
        prefer_encrypt,
    })
}

fn strip_header_name(value: &str) -> &str {
    let rest = value.trim_start();
    for name in ["autocrypt-gossip:", "autocrypt:"] {
        if rest.len() >= name.len() && rest[..name.len()].eq_ignore_ascii_case(name) {
            return rest[name.len()..].trim();
        }
    }
    rest
}

fn prefer(value: &str) -> Option<PreferEncrypt> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mutual" => Some(PreferEncrypt::Mutual),
        "nopreference" => Some(PreferEncrypt::NoPreference),
        _ => None,
    }
}

fn addr_spec(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value
        .strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(value)
        .trim();
    if value.contains(char::is_whitespace) || value.matches('@').count() != 1 {
        return None;
    }
    let (local, domain) = value.split_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn decode_keydata(value: &str) -> Option<Vec<u8>> {
    let mut cleaned: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    if cleaned.is_empty() {
        return None;
    }
    let pad = (4 - cleaned.len() % 4) % 4;
    cleaned.extend(std::iter::repeat_n('=', pad));
    STANDARD.decode(cleaned).ok()
}

fn value_text(value: &HeaderValue<'_>) -> Option<String> {
    match value {
        HeaderValue::Text(text) => Some(text.to_string()),
        HeaderValue::TextList(list) => {
            Some(list.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" "))
        }
        _ => None,
    }
}

fn single_from(message: &mail_parser::Message<'_>) -> Option<String> {
    let mail_parser::Address::List(list) = message.from()? else {
        return None;
    };
    if list.len() != 1 {
        return None;
    }
    let email = list.first()?.address.as_deref()?.trim();
    if email.is_empty() {
        None
    } else {
        Some(email.to_string())
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use super::{PreferEncrypt, autocrypt_header, gossip_keys, parse_autocrypt};

    #[test]
    fn an_autocrypt_header_parses_and_gossip_keys_are_collected() {
        let key = STANDARD.encode(b"ana-key");
        let header =
            format!("Autocrypt: addr=ana@example.com; prefer-encrypt=mutual;\r\n keydata={key}");
        let parsed = parse_autocrypt(&header).unwrap();
        assert_eq!(parsed.addr, "ana@example.com");
        assert_eq!(parsed.keydata, b"ana-key");
        assert_eq!(parsed.prefer_encrypt, PreferEncrypt::Mutual);

        let bob = STANDARD.encode(b"bob-key");
        let cara = STANDARD.encode(b"cara-key");
        let dan = STANDARD.encode(b"dan-key");
        let message = format!(
            "From: Ana <ana@example.com>\r\n\
             {header}\r\n\
             Autocrypt-Gossip: addr=bob@example.com; keydata={bob}\r\n\
             Autocrypt-Gossip: addr=cara@example.com; prefer-encrypt=mutual; keydata={cara}\r\n\
             Autocrypt-Gossip: addr=dan@example.com; type=2; keydata={dan}\r\n\
             Autocrypt-Gossip: addr=erin@example.com; keydata={dan}\r\n\
             \r\n\
             Hi\r\n"
        );
        let from_message = autocrypt_header(message.as_bytes()).unwrap();
        assert_eq!(from_message.addr, "ana@example.com");
        assert_eq!(from_message.keydata, b"ana-key");
        let gossip = gossip_keys(message.as_bytes());
        assert_eq!(gossip.len(), 2);
        assert_eq!(gossip[0].addr, "bob@example.com");
        assert_eq!(gossip[0].keydata, b"bob-key");
        assert_eq!(gossip[1].addr, "erin@example.com");
        assert_eq!(gossip[1].keydata, b"dan-key");
        assert!(parse_autocrypt("Autocrypt: addr=ana@example.com; type=9; keydata=YQ==").is_none());
    }
}
