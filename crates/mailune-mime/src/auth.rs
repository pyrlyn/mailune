//! Authentication-Results badge and DKIM (`rsa-sha256`) verification.
//!
//! `mail-auth` 0.13.3 (crates.io, checked 2026-10-08) is the maintained
//! verifier, but it does not compile unless a DNS client feature is on
//! (`dns-hickory` or `dns-doh`). `mailrs-dkim` 3.1.0 has a resolver trait,
//! and it always links `aws-lc-rs` and verifies asynchronously. Neither fits
//! a TXT record supplied by [`DkimDns`] with no socket. `cfdkim` 0.3.0 was
//! last published in 2023. This module parses the headers itself and checks
//! `rsa-sha256` with RustCrypto `rsa` 0.9.10 (the stable line; 0.10 is still
//! a release candidate) against the `p=` tag. `ed25519-sha256` is reported
//! as [`DkimVerdict::PermError`].

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rsa::RsaPublicKey;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::sha2::{Digest, Sha256};
use rsa::signature::Verifier;

use crate::Error;

/// One method result on an Authentication-Results header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthResult {
    /// `none`.
    None,
    /// `pass`.
    Pass,
    /// `fail`.
    Fail,
    /// `policy`.
    Policy,
    /// `neutral`.
    Neutral,
    /// `temperror`.
    TempError,
    /// `permerror`.
    PermError,
    /// `softfail`.
    SoftFail,
    /// A token this client does not name.
    Other,
}

/// One `method=result` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodBadge {
    /// Method name, lowercased (`dkim`, `spf`, `dmarc`).
    pub method: String,
    /// The result token.
    pub result: AuthResult,
}

/// What a reader shows for Authentication-Results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthBadge {
    /// The authserv-id, without a version number.
    pub server: String,
    /// Methods in header order.
    pub methods: Vec<MethodBadge>,
}

/// Outcome of checking DKIM on one message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DkimVerdict {
    /// At least one `rsa-sha256` signature matched the TXT record.
    Pass,
    /// A signature was present and did not match.
    Fail,
    /// The message has no DKIM-Signature header.
    None,
    /// The trait had no TXT record for a signature that otherwise parsed.
    TempError,
    /// The signature or the TXT record is not one this verifier can use.
    PermError,
}

/// TXT answers for DKIM. The implementor is the only place a name is resolved.
///
/// `name` is `{selector}._domainkey.{domain}`. Return the record the test
/// or the host already holds. `None` means the lookup did not succeed; this
/// crate does not try again.
pub trait DkimDns {
    /// The TXT record at `name`, or `None` when the lookup failed.
    fn txt(&self, name: &str) -> Option<String>;
}

/// Read an Authentication-Results value (or a full header line) into a badge.
pub fn authentication_badge(header: &str) -> Result<AuthBadge, Error> {
    let value = header_value(header);
    let stripped = strip_comments(value);
    let mut sections = stripped.split(';');
    let server = sections
        .next()
        .and_then(|section| section.split_whitespace().next())
        .filter(|token| !token.is_empty())
        .ok_or(Error::AuthenticationResults)?
        .to_string();
    let mut methods = Vec::new();
    for section in sections {
        if let Some((method, result)) = method_from_section(section) {
            methods.push(MethodBadge { method, result });
        }
    }
    Ok(AuthBadge { server, methods })
}

/// Verify DKIM-Signature headers on a CRLF message.
///
/// Only `rsa-sha256` with `c=relaxed/relaxed` is checked. `l=` is refused.
pub fn verify_dkim(message: &[u8], dns: &impl DkimDns) -> DkimVerdict {
    let Some((head, body)) = split_message(message) else {
        return DkimVerdict::PermError;
    };
    let Ok(head) = std::str::from_utf8(head) else {
        return DkimVerdict::PermError;
    };
    let fields = parse_fields(head);
    let signatures: Vec<_> = fields
        .iter()
        .filter(|field| field.name.eq_ignore_ascii_case("dkim-signature"))
        .collect();
    if signatures.is_empty() {
        return DkimVerdict::None;
    }
    let mut saw_temp = false;
    let mut saw_fail = false;
    let mut saw_perm = false;
    for signature in signatures {
        match verify_one(&fields, signature, body, dns) {
            DkimVerdict::Pass => return DkimVerdict::Pass,
            DkimVerdict::TempError => saw_temp = true,
            DkimVerdict::Fail => saw_fail = true,
            DkimVerdict::PermError | DkimVerdict::None => saw_perm = true,
        }
    }
    if saw_temp {
        DkimVerdict::TempError
    } else if saw_fail {
        DkimVerdict::Fail
    } else if saw_perm {
        DkimVerdict::PermError
    } else {
        DkimVerdict::None
    }
}

fn verify_one(
    fields: &[HeaderField],
    signature: &HeaderField,
    body: &[u8],
    dns: &impl DkimDns,
) -> DkimVerdict {
    let Some(tags) = signature_tags(signature) else {
        return DkimVerdict::PermError;
    };
    let get = |name: &str| {
        tags.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    if get("v") != Some("1") || get("a") != Some("rsa-sha256") {
        return DkimVerdict::PermError;
    }
    if let Some(query) = get("q")
        && query != "dns/txt"
    {
        return DkimVerdict::PermError;
    }
    let (Some(domain), Some(selector), Some(signed), Some(body_hash), Some(sig_b)) =
        (get("d"), get("s"), get("h"), get("bh"), get("b"))
    else {
        return DkimVerdict::PermError;
    };
    // Only relaxed/relaxed is implemented. A length limit would hash a prefix
    // and still report pass, so `l=` is refused instead.
    if get("c") != Some("relaxed/relaxed") || get("l").is_some() {
        return DkimVerdict::PermError;
    }
    let canonical_body = relaxed_body(body);
    let actual = STANDARD.encode(Sha256::digest(&canonical_body));
    if !eq_ignore_ws(&actual, body_hash) {
        return DkimVerdict::Fail;
    }
    let name = format!(
        "{}._domainkey.{}",
        selector.to_ascii_lowercase(),
        domain.to_ascii_lowercase()
    );
    let Some(record) = dns.txt(&name) else {
        return DkimVerdict::TempError;
    };
    let Some(key) = public_key(&record) else {
        return DkimVerdict::PermError;
    };
    let Some(signed_bytes) = signed_header(fields, signature, signed) else {
        return DkimVerdict::PermError;
    };
    let Ok(sig_bytes) = STANDARD.decode(strip_ws(sig_b)) else {
        return DkimVerdict::PermError;
    };
    let Ok(parsed) = Signature::try_from(sig_bytes.as_slice()) else {
        return DkimVerdict::PermError;
    };
    let verifying = VerifyingKey::<Sha256>::new(key);
    if verifying.verify(&signed_bytes, &parsed).is_ok() {
        DkimVerdict::Pass
    } else {
        DkimVerdict::Fail
    }
}

fn public_key(record: &str) -> Option<RsaPublicKey> {
    let tags = tag_pairs(&unfold(record));
    let get = |name: &str| {
        tags.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    if let Some(version) = get("v")
        && !version.eq_ignore_ascii_case("DKIM1")
    {
        return None;
    }
    if let Some(kind) = get("k")
        && kind != "rsa"
    {
        return None;
    }
    let p = strip_ws(get("p")?);
    if p.is_empty() {
        return None;
    }
    let der = STANDARD.decode(p).ok()?;
    RsaPublicKey::from_public_key_der(&der)
        .ok()
        .or_else(|| RsaPublicKey::from_pkcs1_der(&der).ok())
}

fn signed_header(
    fields: &[HeaderField],
    signature: &HeaderField,
    signed_names: &str,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut seen: Vec<(String, usize)> = Vec::new();
    for name in signed_names
        .split(':')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let count = seen
            .iter()
            .find(|(seen, _)| seen.eq_ignore_ascii_case(name))
            .map(|(_, count)| *count)
            .unwrap_or(0);
        let field = fields
            .iter()
            .filter(|field| field.name.eq_ignore_ascii_case(name))
            .rev()
            .nth(count);
        if let Some(field) = field {
            out.extend_from_slice(relaxed_field(&field.raw).as_bytes());
            if let Some(slot) = seen
                .iter_mut()
                .find(|(seen, _)| seen.eq_ignore_ascii_case(name))
            {
                slot.1 += 1;
            } else {
                seen.push((name.to_string(), 1));
            }
        }
    }
    out.extend_from_slice(relaxed_field(&empty_b_tag(&signature.raw)?).as_bytes());
    Some(out)
}

fn relaxed_field(raw: &str) -> String {
    let Some((name, value)) = raw.split_once(':') else {
        return String::new();
    };
    let name = name.trim().to_ascii_lowercase();
    let unfolded = value.replace("\r\n", "");
    let mut collapsed = String::new();
    let mut pending_space = false;
    for ch in unfolded.chars() {
        if ch == ' ' || ch == '\t' {
            pending_space = !collapsed.is_empty();
            continue;
        }
        if pending_space {
            collapsed.push(' ');
            pending_space = false;
        }
        collapsed.push(ch);
    }
    format!("{name}:{collapsed}\r\n")
}

fn relaxed_body(body: &[u8]) -> Vec<u8> {
    let mut lines = Vec::new();
    for line in body.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let mut collapsed = Vec::new();
        let mut pending = false;
        for &byte in line {
            if byte == b' ' || byte == b'\t' {
                pending = !collapsed.is_empty();
                continue;
            }
            if pending {
                collapsed.push(b' ');
                pending = false;
            }
            collapsed.push(byte);
        }
        lines.push(collapsed);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(line);
    }
    out.extend_from_slice(b"\r\n");
    out
}

struct HeaderField {
    name: String,
    raw: String,
}

fn parse_fields(head: &str) -> Vec<HeaderField> {
    let mut fields = Vec::new();
    let mut current = String::new();
    for line in head.split("\r\n") {
        if line.starts_with(' ') || line.starts_with('\t') {
            current.push_str("\r\n");
            current.push_str(line);
            continue;
        }
        if !current.is_empty() {
            push_field(&mut fields, &current);
        }
        current = line.to_string();
    }
    if !current.is_empty() {
        push_field(&mut fields, &current);
    }
    fields
}

fn push_field(fields: &mut Vec<HeaderField>, raw: &str) {
    let Some((name, _)) = raw.split_once(':') else {
        return;
    };
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    fields.push(HeaderField {
        name: name.to_string(),
        raw: raw.to_string(),
    });
}

fn signature_tags(field: &HeaderField) -> Option<Vec<(String, String)>> {
    let (_, value) = field.raw.split_once(':')?;
    Some(tag_pairs(&unfold(value)))
}

fn tag_pairs(value: &str) -> Vec<(String, String)> {
    let mut tags = Vec::new();
    for part in value.split(';') {
        let Some((key, raw)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        tags.push((key, raw.trim().to_string()));
    }
    tags
}

fn empty_b_tag(raw: &str) -> Option<String> {
    let (name, rest) = raw.split_once(':')?;
    let mut out = format!("{name}:");
    for (index, part) in rest.split(';').enumerate() {
        if index > 0 {
            out.push(';');
        }
        let trimmed = part.trim_start_matches([' ', '\t', '\r', '\n']);
        let lead = &part[..part.len() - trimmed.len()];
        let key = trimmed.split(['=', ' ', '\t']).next().unwrap_or("");
        let after_key = trimmed.get(key.len()..).unwrap_or("").trim_start();
        if key.eq_ignore_ascii_case("b") && after_key.starts_with('=') {
            out.push_str(lead);
            out.push_str("b=");
        } else {
            out.push_str(part);
        }
    }
    Some(out)
}

fn unfold(value: &str) -> String {
    value.replace("\r\n", "").replace('\n', "")
}

fn strip_ws(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn eq_ignore_ws(left: &str, right: &str) -> bool {
    strip_ws(left) == strip_ws(right)
}

fn split_message(message: &[u8]) -> Option<(&[u8], &[u8])> {
    let split = message
        .windows(4)
        .position(|window| window == b"\r\n\r\n")?;
    Some((&message[..split], &message[split + 4..]))
}

fn header_value(header: &str) -> &str {
    let trimmed = header.trim();
    let prefix = "authentication-results:";
    if trimmed.len() >= prefix.len() && trimmed[..prefix.len()].eq_ignore_ascii_case(prefix) {
        trimmed[prefix.len()..].trim()
    } else {
        trimmed
    }
}

fn strip_comments(input: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && depth > 0 {
            let _ = chars.next();
            continue;
        }
        if ch == '(' {
            depth += 1;
            continue;
        }
        if ch == ')' && depth > 0 {
            depth -= 1;
            continue;
        }
        if depth == 0 {
            out.push(ch);
        }
    }
    out
}

fn method_from_section(section: &str) -> Option<(String, AuthResult)> {
    let head = section.split_whitespace().next()?;
    let (method, result) = head.split_once('=')?;
    if method.contains('.') || method.is_empty() {
        return None;
    }
    Some((method.to_ascii_lowercase(), parse_result(result)))
}

fn parse_result(token: &str) -> AuthResult {
    match token.to_ascii_lowercase().as_str() {
        "none" => AuthResult::None,
        "pass" => AuthResult::Pass,
        "fail" => AuthResult::Fail,
        "policy" => AuthResult::Policy,
        "neutral" => AuthResult::Neutral,
        "temperror" => AuthResult::TempError,
        "permerror" => AuthResult::PermError,
        "softfail" => AuthResult::SoftFail,
        _ => AuthResult::Other,
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use rsa::RsaPrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::pkcs8::EncodePublicKey;
    use rsa::sha2::{Digest, Sha256};
    use rsa::signature::{SignatureEncoding, Signer};

    use super::{
        AuthResult, DkimDns, DkimVerdict, authentication_badge, empty_b_tag, relaxed_body,
        relaxed_field, verify_dkim,
    };

    struct MapDns {
        record: String,
    }

    impl DkimDns for MapDns {
        fn txt(&self, name: &str) -> Option<String> {
            if name == "sel._domainkey.example.com" {
                Some(self.record.clone())
            } else {
                None
            }
        }
    }

    #[test]
    fn authentication_results_become_a_badge() {
        let header = "Authentication-Results: mx.example.com;\r\n dkim=pass header.d=example.com;\r\n spf=softfail;\r\n dmarc=fail (p=reject)";
        let badge = authentication_badge(header).unwrap();
        assert_eq!(badge.server, "mx.example.com");
        assert_eq!(badge.methods.len(), 3);
        assert_eq!(badge.methods[0].method, "dkim");
        assert_eq!(badge.methods[0].result, AuthResult::Pass);
        assert_eq!(badge.methods[1].result, AuthResult::SoftFail);
        assert_eq!(badge.methods[2].result, AuthResult::Fail);
    }

    #[test]
    fn dkim_passes_against_the_supplied_txt_record_and_fails_when_the_body_changes() {
        let mut rng = rand::thread_rng();
        let private = RsaPrivateKey::new(&mut rng, 1024).unwrap();
        let public = rsa::RsaPublicKey::from(&private);
        let der = public.to_public_key_der().unwrap();
        let record = format!("v=DKIM1; k=rsa; p={}", STANDARD.encode(der.as_bytes()));
        let body = b"Hello there\r\n";
        let bh = STANDARD.encode(rsa::sha2::Sha256::digest(relaxed_body(body)));
        let unsigned = format!(
            "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.com; s=sel; h=from:subject; bh={bh}; b="
        );
        let from = "From: Ana <ana@example.com>";
        let subject = "Subject: Hello";
        let mut signed = Vec::new();
        signed.extend_from_slice(relaxed_field(from).as_bytes());
        signed.extend_from_slice(relaxed_field(subject).as_bytes());
        let emptied = empty_b_tag(&unsigned).unwrap();
        signed.extend_from_slice(relaxed_field(&emptied).as_bytes());
        let signature = SigningKey::<Sha256>::new(private).sign(&signed);
        let b64 = STANDARD.encode(signature.to_bytes());
        let message = format!(
            "{unsigned}{b64}\r\n{from}\r\n{subject}\r\n\r\n{}",
            std::str::from_utf8(body).unwrap()
        );
        let dns = MapDns { record };
        assert_eq!(verify_dkim(message.as_bytes(), &dns), DkimVerdict::Pass);
        let tampered = message.replace("Hello there", "Hello There");
        assert_eq!(verify_dkim(tampered.as_bytes(), &dns), DkimVerdict::Fail);
        assert_eq!(
            verify_dkim(b"From: a@b.c\r\n\r\nHi\r\n", &dns),
            DkimVerdict::None
        );
    }

    #[test]
    fn a_missing_txt_record_is_a_temporary_error() {
        let body = b"Hi\r\n";
        let bh = STANDARD.encode(Sha256::digest(relaxed_body(body)));
        let message = format!(
            "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=other.test; s=sel; h=from; bh={bh}; b=YQ==\r\nFrom: a@b.c\r\n\r\nHi\r\n"
        );
        let dns = MapDns {
            record: "v=DKIM1; k=rsa; p=YQ==".to_string(),
        };
        assert_eq!(
            verify_dkim(message.as_bytes(), &dns),
            DkimVerdict::TempError
        );
    }
}
