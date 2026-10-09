//! Gmail's batch endpoint: many GETs in one `multipart/mixed` request, and
//! the `multipart/mixed` answer split back into status and body per part.

use crate::Error;

/// Boundary for requests. Gmail picks its own for the response.
pub(crate) const BOUNDARY: &str = "mailune_batch";

/// One inner response.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Part {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

/// Rejects an id that could change the request path. Ids come from the
/// server and are untrusted.
pub(crate) fn segment(id: &str) -> Result<&str, Error> {
    let safe = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if safe {
        Ok(id)
    } else {
        Err(Error::Format(
            "id has characters outside [A-Za-z0-9_-]".into(),
        ))
    }
}

/// A batch body of one `GET` per path. Paths are already validated.
pub(crate) fn encode(paths: &[String]) -> Vec<u8> {
    let mut body = String::new();
    for (index, path) in paths.iter().enumerate() {
        body.push_str(&format!(
            "--{BOUNDARY}\r\nContent-Type: application/http\r\nContent-ID: <item{index}>\r\n\r\nGET {path}\r\n\r\n"
        ));
    }
    body.push_str(&format!("--{BOUNDARY}--\r\n"));
    body.into_bytes()
}

/// Splits a batch response. `content_type` is the outer header, which names
/// the boundary.
pub(crate) fn decode(content_type: &str, body: &[u8]) -> Result<Vec<Part>, Error> {
    let boundary = boundary(content_type)
        .ok_or_else(|| Error::Format("batch response has no boundary".into()))?;
    let text = std::str::from_utf8(body)
        .map_err(|_| Error::Format("batch response is not UTF-8".into()))?;
    let delimiter = format!("--{boundary}");
    let mut parts = Vec::new();
    // The first piece is the preamble; a piece starting with `--` closes the body.
    for piece in text.split(delimiter.as_str()).skip(1) {
        if piece.starts_with("--") {
            break;
        }
        parts.push(inner(piece)?);
    }
    Ok(parts)
}

fn boundary(content_type: &str) -> Option<&str> {
    content_type.split(';').find_map(|param| {
        let (key, value) = param.trim().split_once('=')?;
        key.eq_ignore_ascii_case("boundary")
            .then(|| value.trim().trim_matches('"'))
            .filter(|value| !value.is_empty())
    })
}

fn inner(piece: &str) -> Result<Part, Error> {
    let bad = || Error::Format("batch part is not an HTTP response".into());
    // Part headers, a blank line, then the embedded response.
    let (_, response) = split_blank(piece).ok_or_else(bad)?;
    let response = response.trim_start_matches(['\r', '\n']);
    let (head, body) = split_blank(response).unwrap_or((response, ""));
    let status_line = head.lines().next().ok_or_else(bad)?;
    let mut words = status_line.split_whitespace();
    if !words.next().is_some_and(|proto| proto.starts_with("HTTP/")) {
        return Err(bad());
    }
    let status = words
        .next()
        .and_then(|code| code.parse().ok())
        .ok_or_else(bad)?;
    Ok(Part {
        status,
        body: body.trim_end_matches(['\r', '\n']).as_bytes().to_vec(),
    })
}

fn split_blank(text: &str) -> Option<(&str, &str)> {
    match (text.find("\r\n\r\n"), text.find("\n\n")) {
        (Some(crlf), Some(lf)) if lf < crlf => Some((&text[..lf], &text[lf + 2..])),
        (Some(crlf), _) => Some((&text[..crlf], &text[crlf + 4..])),
        (None, Some(lf)) => Some((&text[..lf], &text[lf + 2..])),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, segment};

    #[test]
    fn a_request_names_each_get() {
        let body = String::from_utf8(encode(&["/a".into(), "/b".into()])).unwrap();
        assert!(body.contains("Content-ID: <item1>\r\n\r\nGET /b\r\n"));
        assert!(body.ends_with("--mailune_batch--\r\n"));
    }

    #[test]
    fn a_response_splits_into_status_and_body() {
        let body = "--batch_x\r\nContent-Type: application/http\r\nContent-ID: <response-item0>\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"id\":\"a\"}\r\n--batch_x\r\nContent-Type: application/http\r\n\r\nHTTP/1.1 404 Not Found\r\n\r\n{}\r\n--batch_x--\r\n";
        let parts = decode("multipart/mixed; boundary=\"batch_x\"", body.as_bytes()).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(
            (parts[0].status, parts[0].body.as_slice()),
            (200, &b"{\"id\":\"a\"}"[..])
        );
        assert_eq!(parts[1].status, 404);
        assert!(decode("multipart/mixed", body.as_bytes()).is_err());
        assert!(
            decode(
                "multipart/mixed; boundary=batch_x",
                b"--batch_x\r\n\r\nnot http\r\n--batch_x--"
            )
            .is_err()
        );
    }

    #[test]
    fn ids_that_could_change_the_path_are_refused() {
        assert!(segment("18c2f0a1b2").is_ok());
        for bad in ["", "../labels", "a?b", "a/b", "a b"] {
            assert!(segment(bad).is_err(), "{bad}");
        }
    }
}
