//! Screening of webhook bodies. The relay passes on only that something
//! changed, so a body that carries a credential or mail content is refused
//! rather than trimmed: such a body means the provider subscription was set
//! up to send more than the relay may see.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;

/// Largest body the relay reads. Change notifications are small.
pub const MAX_BODY: usize = 16 * 1024;

/// Keys that name mail content. Matched without case.
const CONTENT_KEYS: [&str; 10] = [
    "subject",
    "body",
    "bodypreview",
    "uniquebody",
    "snippet",
    "raw",
    "payload",
    "content",
    "encryptedcontent",
    "torecipients",
];

/// The fields Gmail's Pub/Sub `data` may carry.
const GMAIL_DATA_KEYS: [&str; 2] = ["emailAddress", "historyId"];

/// Who sent the webhook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    /// Gmail through a Cloud Pub/Sub push subscription.
    Gmail,
    /// Microsoft Graph change notifications.
    Graph,
}

/// Why a body was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Over [`MAX_BODY`].
    TooLarge,
    /// Not the JSON shape the provider sends.
    Shape,
    /// A key that names a token or mail content.
    Forbidden(String),
    /// A string value that looks like a bearer token.
    Credential,
}

impl Provider {
    /// The provider named in a webhook path.
    pub fn from_path(segment: &str) -> Option<Self> {
        match segment {
            "gmail" => Some(Self::Gmail),
            "graph" => Some(Self::Graph),
            _ => None,
        }
    }
}

/// Accepts a body that only says something changed.
///
/// # Errors
///
/// The [`Refusal`] that applies first.
pub fn screen(provider: Provider, body: &[u8]) -> Result<(), Refusal> {
    if body.len() > MAX_BODY {
        return Err(Refusal::TooLarge);
    }
    let json: Value = serde_json::from_slice(body).map_err(|_| Refusal::Shape)?;
    walk(&json)?;
    match provider {
        Provider::Gmail => gmail(&json),
        Provider::Graph => match json.get("value") {
            Some(Value::Array(items)) if items.iter().all(Value::is_object) => Ok(()),
            _ => Err(Refusal::Shape),
        },
    }
}

fn gmail(json: &Value) -> Result<(), Refusal> {
    let data = json
        .get("message")
        .and_then(|message| message.get("data"))
        .and_then(Value::as_str)
        .ok_or(Refusal::Shape)?;
    let decoded = STANDARD.decode(data).map_err(|_| Refusal::Shape)?;
    let inner: Value = serde_json::from_slice(&decoded).map_err(|_| Refusal::Shape)?;
    let object = inner.as_object().ok_or(Refusal::Shape)?;
    if let Some(key) = object
        .keys()
        .find(|key| !GMAIL_DATA_KEYS.contains(&key.as_str()))
    {
        return Err(Refusal::Forbidden(key.clone()));
    }
    walk(&inner)
}

fn walk(value: &Value) -> Result<(), Refusal> {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let lower = key.to_ascii_lowercase();
                if lower.contains("token") || CONTENT_KEYS.contains(&lower.as_str()) {
                    return Err(Refusal::Forbidden(key.clone()));
                }
                walk(inner)?;
            }
            Ok(())
        }
        Value::Array(items) => items.iter().try_for_each(walk),
        Value::String(text) if looks_like_credential(text) => Err(Refusal::Credential),
        _ => Ok(()),
    }
}

/// Bearer headers, Google access tokens, and JWTs.
fn looks_like_credential(text: &str) -> bool {
    let text = text.trim();
    let jwt = text.starts_with("eyJ") && text.split('.').count() == 3;
    jwt || text.starts_with("ya29.") || text.to_ascii_lowercase().starts_with("bearer ")
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;

    use super::{MAX_BODY, Provider, Refusal, screen};

    fn gmail(data: &serde_json::Value) -> Vec<u8> {
        let data = STANDARD.encode(data.to_string());
        json!({
            "message": { "data": data, "messageId": "2070443601311540", "publishTime": "2026-10-08T10:00:00Z" },
            "subscription": "projects/mailune/subscriptions/relay"
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn a_change_notice_passes() {
        let notice = gmail(&json!({ "emailAddress": "me@gmail.com", "historyId": "9876" }));
        assert_eq!(screen(Provider::Gmail, &notice), Ok(()));
        let graph = json!({ "value": [{
            "subscriptionId": "sub-1", "changeType": "created", "clientState": "opaque",
            "resource": "Users/u1/Messages/m1",
            "resourceData": { "@odata.type": "#Microsoft.Graph.Message", "id": "m1" }
        }]});
        assert_eq!(
            screen(Provider::Graph, graph.to_string().as_bytes()),
            Ok(())
        );
    }

    #[test]
    fn tokens_and_mail_content_are_refused() {
        let extra = gmail(&json!({ "emailAddress": "a@b", "historyId": "1", "snippet": "hi" }));
        assert_eq!(
            screen(Provider::Gmail, &extra),
            Err(Refusal::Forbidden("snippet".into()))
        );
        let rich = json!({ "value": [{ "subscriptionId": "s",
            "encryptedContent": { "data": "x" }, "validationTokens": ["eyJ.a.b"] }]});
        assert!(matches!(
            screen(Provider::Graph, rich.to_string().as_bytes()),
            Err(Refusal::Forbidden(_))
        ));
        let subject = json!({ "value": [{ "resourceData": { "Subject": "Payroll" } }] });
        assert_eq!(
            screen(Provider::Graph, subject.to_string().as_bytes()),
            Err(Refusal::Forbidden("Subject".into()))
        );
        let bearer = json!({ "value": [{ "clientState": "Bearer abc" }] });
        assert_eq!(
            screen(Provider::Graph, bearer.to_string().as_bytes()),
            Err(Refusal::Credential)
        );
        // A fake Google-shaped token: the screen must refuse exactly that shape.
        // Semgrep's generic rules read nosemgrep only on the matched line.
        let access = json!({ "message": { "data": "e30=", "access_token": "ya29.x" } }); // nosemgrep
        assert!(screen(Provider::Gmail, access.to_string().as_bytes()).is_err());
    }

    #[test]
    fn odd_shapes_and_large_bodies_are_refused() {
        assert_eq!(screen(Provider::Graph, b"not json"), Err(Refusal::Shape));
        assert_eq!(screen(Provider::Graph, b"{}"), Err(Refusal::Shape));
        assert_eq!(
            screen(Provider::Gmail, br#"{"message":{"data":"!!"}}"#),
            Err(Refusal::Shape)
        );
        let large = vec![b' '; MAX_BODY + 1];
        assert_eq!(screen(Provider::Gmail, &large), Err(Refusal::TooLarge));
    }
}
