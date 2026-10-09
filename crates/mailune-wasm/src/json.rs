//! The JSON shapes the browser sees. Domain types stay free of serde; the
//! encoding for JavaScript lives only here.

use mailune_core::{Container, Term, Threadable};
use mailune_mime::{MimeMessage, PartRole};
use mailune_protocol::{Envelope, MailboxId};
use serde::Deserialize;
use serde_json::{Value, json};

/// A message for threading, as the browser sends it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadInput {
    message_id: String,
    #[serde(default)]
    in_reply_to: Option<String>,
    #[serde(default)]
    references: Vec<String>,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    provider_thread: Option<String>,
}

pub(crate) fn envelope(text: &str) -> Result<String, String> {
    let envelope: Envelope = serde_json::from_str(text).map_err(|error| error.to_string())?;
    serde_json::to_string(&envelope).map_err(|error| error.to_string())
}

pub(crate) fn mailbox_ids(ids: &[MailboxId]) -> String {
    Value::Array(
        ids.iter()
            .map(|id| Value::String(id.as_str().to_string()))
            .collect(),
    )
    .to_string()
}

pub(crate) fn mime(message: &MimeMessage) -> String {
    let parts: Vec<Value> = message
        .parts
        .iter()
        .map(|part| {
            json!({
                "role": role(part.role),
                "mediaType": part.media_type,
                "filename": part.filename,
            })
        })
        .collect();
    json!({
        "subject": message.subject,
        "from": message.from,
        "to": message.to,
        "cc": message.cc,
        "messageId": message.message_id,
        "inReplyTo": message.in_reply_to,
        "references": message.references,
        "parts": parts,
    })
    .to_string()
}

fn role(role: PartRole) -> &'static str {
    match role {
        PartRole::Text => "text",
        PartRole::Html => "html",
        PartRole::Attachment => "attachment",
        PartRole::Inline => "inline",
        PartRole::Other => "other",
    }
}

pub(crate) fn threadables(text: &str) -> Result<Vec<Threadable>, String> {
    let input: Vec<ThreadInput> = serde_json::from_str(text).map_err(|error| error.to_string())?;
    Ok(input
        .into_iter()
        .map(|message| Threadable {
            message_id: message.message_id,
            in_reply_to: message.in_reply_to,
            references: message.references,
            subject: message.subject,
            provider_thread: message.provider_thread,
        })
        .collect())
}

pub(crate) fn threads(roots: &[Container]) -> String {
    Value::Array(roots.iter().map(container).collect()).to_string()
}

fn container(node: &Container) -> Value {
    json!({
        "messageId": node.message_id,
        "children": node.children.iter().map(container).collect::<Vec<_>>(),
    })
}

pub(crate) fn terms(terms: &[Term]) -> String {
    let terms: Vec<Value> = terms
        .iter()
        .map(|term| match term {
            Term::From(value) => json!({ "from": value }),
            Term::To(value) => json!({ "to": value }),
            Term::HasAttachment => json!({ "hasAttachment": true }),
            Term::Before(date) => json!({
                "before": format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
            }),
            Term::Unread => json!({ "unread": true }),
            Term::Label(value) => json!({ "label": value }),
            Term::Text(value) => json!({ "text": value }),
        })
        .collect();
    Value::Array(terms).to_string()
}
