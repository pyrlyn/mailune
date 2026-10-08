//! JMAP push frames, without a socket.
//!
//! An EventSource `data:` block (RFC 8620) and a WebSocket text frame
//! (RFC 8887) both carry a `StateChange`. The bytes are already in hand.

use serde_json::Value;

use crate::Error;

/// One type's new state inside a push `StateChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateChange {
    /// Account whose state moved.
    pub account_id: String,
    /// JMAP type name (`Email`, `Mailbox`, …).
    pub type_name: String,
    /// The new state string.
    pub state: String,
}

/// Parses one EventSource message. Comment and non-`state` events are ignored.
///
/// # Errors
///
/// [`Error::Json`] or [`Error::Body`] when a dispatched `data` block is not a
/// `changed` object.
pub fn parse_event_source(frame: &str) -> Result<Vec<StateChange>, Error> {
    let mut out = Vec::new();
    for data in event_data(frame) {
        let value: Value =
            serde_json::from_str(&data).map_err(|err| Error::Json(err.to_string()))?;
        out.extend(changes_from_value(&value)?);
    }
    Ok(out)
}

/// Parses one WebSocket text frame. RFC 8887 requires `@type` of `StateChange`.
///
/// # Errors
///
/// [`Error::Json`] or [`Error::Body`] when the frame is not that object.
pub fn parse_websocket(frame: &str) -> Result<Vec<StateChange>, Error> {
    let value: Value = serde_json::from_str(frame).map_err(|err| Error::Json(err.to_string()))?;
    let kind = value
        .get("@type")
        .and_then(Value::as_str)
        .ok_or(Error::Body)?;
    if kind != "StateChange" {
        return Err(Error::Body);
    }
    changes_from_value(&value)
}

fn event_data(frame: &str) -> Vec<String> {
    let mut data = String::new();
    let mut event = String::new();
    let mut out = Vec::new();
    for line in frame.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            take_data(&mut out, &mut event, &mut data);
            continue;
        }
        if line.starts_with(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        } else if let Some(rest) = line.strip_prefix("event:") {
            event = rest.trim().to_string();
        }
    }
    take_data(&mut out, &mut event, &mut data);
    out
}

fn take_data(out: &mut Vec<String>, event: &mut String, data: &mut String) {
    if data.is_empty() {
        event.clear();
        return;
    }
    if event.is_empty() || event == "state" {
        out.push(std::mem::take(data));
    } else {
        data.clear();
    }
    event.clear();
}

fn changes_from_value(value: &Value) -> Result<Vec<StateChange>, Error> {
    let changed = value
        .get("changed")
        .and_then(Value::as_object)
        .ok_or(Error::Body)?;
    let mut out = Vec::new();
    for (account_id, types) in changed {
        let types = types.as_object().ok_or(Error::Body)?;
        for (type_name, state) in types {
            let state = state.as_str().ok_or(Error::Body)?;
            out.push(StateChange {
                account_id: account_id.clone(),
                type_name: type_name.clone(),
                state: state.to_string(),
            });
        }
    }
    out.sort_by(|left, right| {
        (&left.account_id, &left.type_name).cmp(&(&right.account_id, &right.type_name))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{parse_event_source, parse_websocket};

    const CHANGED: &str = r#"{"changed":{"acc-1":{"Email":"s2","Mailbox":"mb2"}}}"#;

    #[test]
    fn event_source_and_websocket_are_the_same_change() {
        let sse = format!(": keep-alive\nevent: state\ndata: {CHANGED}\n\n");
        let ws = r#"{"@type":"StateChange","changed":{"acc-1":{"Mailbox":"mb2","Email":"s2"}}}"#;
        let from_sse = parse_event_source(&sse).unwrap();
        let from_ws = parse_websocket(ws).unwrap();
        assert_eq!(from_sse, from_ws);
        assert_eq!(from_sse.len(), 2);
        assert_eq!(from_sse[0].type_name, "Email");
        assert_eq!(from_sse[0].state, "s2");
        assert_eq!(from_sse[1].type_name, "Mailbox");
        assert_eq!(from_sse[1].state, "mb2");
        assert_eq!(from_sse[0].account_id, "acc-1");
    }

    #[test]
    fn a_non_state_event_is_skipped() {
        let frame = "event: ping\ndata: {\"ok\":true}\n\n";
        assert!(parse_event_source(frame).unwrap().is_empty());
    }

    #[test]
    fn a_websocket_request_is_not_a_state_change() {
        let err = parse_websocket(r#"{"@type":"RequestTooLarge"}"#).unwrap_err();
        assert!(matches!(err, crate::Error::Body));
    }
}
