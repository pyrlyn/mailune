//! Graph calendar availability and contact autocomplete.
//!
//! `getSchedule` and the people search are scripted JSON. Nothing here opens
//! a socket. Contacts land in the store. Busy blocks have no table, so they
//! stay a typed result.

use serde_json::Value;

use mailune_store::{AccountRow, ContactRow, Store};

use crate::Error;

/// One busy interval from `getSchedule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusyBlock {
    /// `scheduleId`, usually an email address.
    pub schedule_id: String,
    /// `status` (`busy`, `tentative`, `oof`).
    pub status: String,
    /// Start, as the service spelled it.
    pub start: String,
    /// End, as the service spelled it.
    pub end: String,
}

/// One address from people autocomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactHit {
    /// `displayName`.
    pub name: String,
    /// Email address.
    pub email: String,
}

/// Parses a `getSchedule` response into busy blocks.
///
/// # Errors
///
/// [`Error::Json`] or [`Error::Body`] when `value` is missing.
pub fn availability(json: &str) -> Result<Vec<BusyBlock>, Error> {
    let doc: Value = parse(json)?;
    let people = doc
        .get("value")
        .and_then(Value::as_array)
        .ok_or(Error::Body)?;
    let mut out = Vec::new();
    for person in people {
        let schedule_id = person
            .get("scheduleId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let Some(items) = person.get("scheduleItems").and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            out.push(BusyBlock {
                schedule_id: schedule_id.clone(),
                status: item
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                start: when(item.get("start")),
                end: when(item.get("end")),
            });
        }
    }
    Ok(out)
}

/// Parses people autocomplete and upserts each address as a contact.
///
/// An account that is already stored keeps its username. A missing account is
/// inserted so the contact's foreign key has a parent.
///
/// # Errors
///
/// [`Error::Body`] when `value` is missing. [`Error::Store`] when a contact
/// cannot be written.
pub fn autocomplete(
    store: &mut Store,
    account_id: &str,
    json: &str,
) -> Result<Vec<ContactHit>, Error> {
    let doc: Value = parse(json)?;
    let people = doc
        .get("value")
        .and_then(Value::as_array)
        .ok_or(Error::Body)?;
    let mut hits = Vec::new();
    for person in people {
        let name = person
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        for email in emails_of(person) {
            hits.push(ContactHit {
                name: name.clone(),
                email,
            });
        }
    }
    if store.account(account_id)?.is_none() {
        let email = hits
            .first()
            .map(|hit| hit.email.clone())
            .unwrap_or_default();
        if !email.is_empty() {
            store.upsert_account(&AccountRow {
                id: account_id.to_string(),
                email,
            })?;
        }
    }
    for hit in &hits {
        store.upsert_contact(&ContactRow {
            account_id: account_id.to_string(),
            email: hit.email.clone(),
            name: hit.name.clone(),
        })?;
    }
    Ok(hits)
}

fn emails_of(person: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["scoredEmailAddresses", "emailAddresses"] {
        let Some(list) = person.get(key).and_then(Value::as_array) else {
            continue;
        };
        for item in list {
            let Some(address) = item.get("address").and_then(Value::as_str) else {
                continue;
            };
            if !address.is_empty() && !out.iter().any(|have| have == address) {
                out.push(address.to_string());
            }
        }
    }
    out
}

fn when(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    value
        .get("dateTime")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn parse(json: &str) -> Result<Value, Error> {
    serde_json::from_str(json).map_err(|err| Error::Json(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{autocomplete, availability};
    use mailune_store::{AccountRow, Store};

    #[test]
    fn availability_and_autocomplete_parse_the_script() {
        let blocks = availability(
            r#"{"value":[{"scheduleId":"ada@example.com","availabilityView":"2","scheduleItems":[{"status":"busy","start":{"dateTime":"2026-03-01T12:00:00.0000000","timeZone":"UTC"},"end":{"dateTime":"2026-03-01T13:00:00.0000000","timeZone":"UTC"}}]}]}"#,
        )
        .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].schedule_id, "ada@example.com");
        assert_eq!(blocks[0].status, "busy");
        assert_eq!(blocks[0].start, "2026-03-01T12:00:00.0000000");
        assert_eq!(blocks[0].end, "2026-03-01T13:00:00.0000000");

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "mailune-graph-people-{nanos}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        store
            .upsert_account(&AccountRow {
                id: "acc-1".into(),
                email: "me@example.com".into(),
            })
            .unwrap();
        let hits = autocomplete(
            &mut store,
            "acc-1",
            r#"{"value":[{"displayName":"Ada Lovelace","scoredEmailAddresses":[{"address":"ada@example.com","relevanceScore":1}],"emailAddresses":[{"address":"ada@example.com"}]}]}"#,
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Ada Lovelace");
        assert_eq!(hits[0].email, "ada@example.com");
        let account = store.account("acc-1").unwrap().unwrap();
        assert_eq!(account.email, "me@example.com");
        let _ = std::fs::remove_dir_all(dir);
    }
}
