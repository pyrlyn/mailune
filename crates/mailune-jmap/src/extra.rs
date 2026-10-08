//! Masked Email and Sieve from a scripted exchange.
//!
//! RFC 9661 lists `SieveScript` objects. The Masked Email extension creates an
//! address. Nothing here opens a socket.

use serde_json::Value;

use mailune_store::{AccountRow, ContactRow, Store};

use crate::Error;
use crate::mutate::method_body;

/// A Masked Email the server created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskedEmail {
    /// Server id.
    pub id: String,
    /// The generated address.
    pub email: String,
    /// Domain the address was created for.
    pub for_domain: String,
}

/// One Sieve script from `SieveScript/get`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SieveScript {
    /// Script id.
    pub id: String,
    /// Script name.
    pub name: String,
    /// `isActive`.
    pub active: bool,
}

/// Creates a Masked Email and stores the address as a contact.
///
/// The contact's foreign key needs an account row. An account that is already
/// stored keeps its username; a missing one is inserted so the contact can land.
///
/// # Errors
///
/// [`Error::Body`] when `created` has no address. [`Error::Store`] when the
/// contact cannot be written.
pub fn create_masked(
    store: &mut Store,
    request: &str,
    response: &str,
) -> Result<Vec<MaskedEmail>, Error> {
    let request = method_body(request, "methodCalls", "MaskedEmail/set")?;
    let response = method_body(response, "methodResponses", "MaskedEmail/set")?;
    if response
        .get("notCreated")
        .and_then(Value::as_object)
        .is_some_and(|map| !map.is_empty())
    {
        return Err(Error::Body);
    }
    let account_id = request
        .get("accountId")
        .and_then(Value::as_str)
        .ok_or(Error::Body)?;
    let create = request
        .get("create")
        .and_then(Value::as_object)
        .ok_or(Error::Body)?;
    let created = response
        .get("created")
        .and_then(Value::as_object)
        .ok_or(Error::Body)?;
    let mut out = Vec::new();
    for (client_id, made) in created {
        let id = made.get("id").and_then(Value::as_str).ok_or(Error::Body)?;
        let email = made
            .get("email")
            .and_then(Value::as_str)
            .ok_or(Error::Body)?;
        let for_domain = create
            .get(client_id)
            .and_then(|row| row.get("forDomain"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if store.account(account_id)?.is_none() {
            store.upsert_account(&AccountRow {
                id: account_id.to_string(),
                email: email.to_string(),
            })?;
        }
        store.upsert_contact(&ContactRow {
            account_id: account_id.to_string(),
            email: email.to_string(),
            name: for_domain.clone(),
        })?;
        out.push(MaskedEmail {
            id: id.to_string(),
            email: email.to_string(),
            for_domain,
        });
    }
    Ok(out)
}

/// Lists Sieve scripts from a `SieveScript/get` response.
///
/// # Errors
///
/// [`Error::Body`] when `list` is missing or a script has no id or name.
pub fn list_sieve(response: &str) -> Result<Vec<SieveScript>, Error> {
    let body = method_body(response, "methodResponses", "SieveScript/get")?;
    let list = body
        .get("list")
        .and_then(Value::as_array)
        .ok_or(Error::Body)?;
    let mut out = Vec::new();
    for item in list {
        let id = item.get("id").and_then(Value::as_str).ok_or(Error::Body)?;
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .ok_or(Error::Body)?;
        let active = item
            .get("isActive")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        out.push(SieveScript {
            id: id.to_string(),
            name: name.to_string(),
            active,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{create_masked, list_sieve};
    use mailune_store::Store;

    #[test]
    fn masked_email_is_created_and_a_sieve_script_is_listed() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("mailune-jmap-extra-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut store = Store::open(&dir.join("mail.db"), b"key").unwrap();
        let created = create_masked(
            &mut store,
            r#"{"methodCalls":[["MaskedEmail/set",{"accountId":"acc-1","create":{"k1":{"forDomain":"shop.example"}}},"0"]]}"#,
            r#"{"methodResponses":[["MaskedEmail/set",{"created":{"k1":{"id":"me-1","email":"abc@masked.example"}}},"0"]]}"#,
        )
        .unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].id, "me-1");
        assert_eq!(created[0].email, "abc@masked.example");
        assert_eq!(created[0].for_domain, "shop.example");
        let account = store.account("acc-1").unwrap().unwrap();
        assert_eq!(account.email, "abc@masked.example");

        let scripts = list_sieve(
            r#"{"methodResponses":[["SieveScript/get",{"accountId":"acc-1","list":[{"id":"sv1","name":"filter","isActive":true}]},"0"]]}"#,
        )
        .unwrap();
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].id, "sv1");
        assert_eq!(scripts[0].name, "filter");
        assert!(scripts[0].active);
        let _ = std::fs::remove_dir_all(dir);
    }
}
