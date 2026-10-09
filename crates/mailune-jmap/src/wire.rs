//! The JMAP request envelope: `using`, `methodCalls`, `methodResponses`,
//! and result references between calls in one request.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::Error;

/// Capabilities every mail request declares.
pub(crate) const USING: [&str; 2] = ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"];

#[derive(Serialize)]
struct Request<'a> {
    using: &'a [&'a str],
    #[serde(rename = "methodCalls")]
    method_calls: Vec<(&'a str, &'a Value, String)>,
}

#[derive(Deserialize)]
struct Response {
    #[serde(rename = "methodResponses")]
    method_responses: Vec<(String, Value, String)>,
}

/// Serialises calls with ids `c0`, `c1`, … in order.
pub(crate) fn encode(using: &[&str], calls: &[(&str, Value)]) -> Result<Vec<u8>, Error> {
    let request = Request {
        using,
        method_calls: calls
            .iter()
            .enumerate()
            .map(|(index, (name, args))| (*name, args, format!("c{index}")))
            .collect(),
    };
    serde_json::to_vec(&request).map_err(json_error)
}

/// The arguments of each response, in call order. A server `error`
/// response for any call fails the whole batch.
pub(crate) fn decode(body: &[u8]) -> Result<Vec<(String, Value)>, Error> {
    let response: Response = serde_json::from_slice(body).map_err(json_error)?;
    let mut out = Vec::with_capacity(response.method_responses.len());
    for (name, args, _) in response.method_responses {
        if name == "error" {
            let kind = args
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            return Err(Error::Method { kind });
        }
        out.push((name, args));
    }
    Ok(out)
}

/// The arguments of the response named `name`, decoded into `T`.
pub(crate) fn take<T: for<'de> Deserialize<'de>>(
    responses: &[(String, Value)],
    name: &str,
) -> Result<T, Error> {
    let (_, args) = responses
        .iter()
        .find(|(have, _)| have == name)
        .ok_or_else(|| Error::Json(format!("missing {name} response")))?;
    T::deserialize(args).map_err(json_error)
}

/// A result reference (`#ids`) to `path` in the response of call `index`.
pub(crate) fn reference(index: usize, name: &str, path: &str) -> Value {
    json!({ "resultOf": format!("c{index}"), "name": name, "path": path })
}

pub(crate) fn json_error(err: serde_json::Error) -> Error {
    Error::Json(err.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{USING, decode, encode, reference};
    use crate::Error;

    #[test]
    fn calls_get_sequential_ids_and_back_references() {
        let body = encode(
            &USING,
            &[
                ("Email/query", json!({ "accountId": "a" })),
                (
                    "Email/get",
                    json!({ "#ids": reference(0, "Email/query", "/ids") }),
                ),
            ],
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["methodCalls"][1][2], "c1");
        assert_eq!(value["methodCalls"][1][1]["#ids"]["resultOf"], "c0");
        assert_eq!(value["using"][1], "urn:ietf:params:jmap:mail");
    }

    #[test]
    fn a_method_error_fails_the_batch() {
        let body = br#"{"methodResponses":[["error",{"type":"cannotCalculateChanges"},"c0"]],"sessionState":"x"}"#;
        assert!(matches!(
            decode(body),
            Err(Error::Method { kind }) if kind == "cannotCalculateChanges"
        ));
    }
}
