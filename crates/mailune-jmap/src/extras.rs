//! Provider extensions: Fastmail's MaskedEmail and JMAP for Sieve (RFC 9661).

use std::collections::BTreeMap;

use mailune_protocol::Http;
use serde::Deserialize;
use serde_json::json;

use crate::mail::GetResult;
use crate::wire::{USING, take};
use crate::{Error, JmapClient};

/// Fastmail's capability; see <https://www.fastmail.com/dev/>.
const MASKED: &str = "https://www.fastmail.com/dev/maskedemail";
/// RFC 9661.
const SIEVE: &str = "urn:ietf:params:jmap:sieve";

/// A masked address the server created.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskedEmail {
    /// Server id.
    pub id: String,
    /// The generated address.
    pub email: String,
    /// `pending`, `enabled`, `disabled` or `deleted`.
    pub state: String,
    /// Site the address was made for, when given.
    #[serde(default)]
    pub for_domain: Option<String>,
}

/// One Sieve script on the server.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SieveScript {
    /// Server id.
    pub id: String,
    /// Script name, if set.
    #[serde(default)]
    pub name: Option<String>,
    /// Blob holding the script text.
    pub blob_id: String,
    /// The active script filters incoming mail.
    pub is_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaskedSet {
    #[serde(default)]
    created: BTreeMap<String, MaskedEmail>,
    #[serde(default)]
    not_created: BTreeMap<String, Refusal>,
}

#[derive(Deserialize)]
struct Refusal {
    #[serde(rename = "type")]
    kind: String,
}

impl<H: Http> JmapClient<'_, H> {
    /// Creates an enabled masked address for `domain`.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] when the server refuses, [`Error::Method`] when the
    /// account has no MaskedEmail capability.
    pub async fn create_masked_email(
        &self,
        domain: &str,
        description: &str,
    ) -> Result<MaskedEmail, Error> {
        let account = self.account()?;
        let responses = self
            .call_using(
                &[USING[0], MASKED],
                &[(
                    "MaskedEmail/set",
                    json!({
                        "accountId": account,
                        "create": { "new": {
                            "forDomain": domain,
                            "description": description,
                            "state": "enabled",
                        } },
                    }),
                )],
            )
            .await?;
        let mut set: MaskedSet = take(&responses, "MaskedEmail/set")?;
        if let Some(refusal) = set.not_created.remove("new") {
            return Err(Error::Rejected { kind: refusal.kind });
        }
        set.created
            .remove("new")
            .ok_or_else(|| Error::Json("MaskedEmail/set created nothing".into()))
    }

    /// Every Sieve script on the account.
    ///
    /// # Errors
    ///
    /// [`Error::Method`] when the account has no Sieve capability.
    pub async fn sieve_scripts(&self) -> Result<Vec<SieveScript>, Error> {
        let account = self.account()?;
        let responses = self
            .call_using(
                &[USING[0], SIEVE],
                &[(
                    "SieveScript/get",
                    json!({ "accountId": account, "ids": null }),
                )],
            )
            .await?;
        let got: GetResult<SieveScript> = take(&responses, "SieveScript/get")?;
        Ok(got.list)
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::Secret;
    use mailune_testkit::{ScriptedHttp, poll_now};
    use serde_json::Value;

    use crate::{Error, JmapClient};

    const SESSION: &str = include_str!("../fixtures/session.json");

    #[test]
    fn a_masked_email_is_created_and_a_refusal_is_typed() {
        let http = ScriptedHttp::new()
            .json(SESSION)
            .json(r#"{"methodResponses":[["MaskedEmail/set",{"accountId":"acc-1","created":{"new":{"id":"me-1","email":"blue.fox1234@fastmail.com","state":"enabled","forDomain":"https://shop.example"}}},"c0"]]}"#)
            .json(r#"{"methodResponses":[["MaskedEmail/set",{"accountId":"acc-1","notCreated":{"new":{"type":"overQuota"}}},"c0"]]}"#);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let masked = poll_now(client.create_masked_email("https://shop.example", "shop"))
            .unwrap()
            .unwrap();
        assert_eq!(masked.email, "blue.fox1234@fastmail.com");
        assert_eq!(masked.state, "enabled");
        let sent: Value = serde_json::from_slice(&http.requests()[1].body).unwrap();
        assert_eq!(sent["using"][1], "https://www.fastmail.com/dev/maskedemail");
        assert_eq!(
            sent["methodCalls"][0][1]["create"]["new"]["forDomain"],
            "https://shop.example"
        );
        assert!(matches!(
            poll_now(client.create_masked_email("https://other.example", "x")).unwrap(),
            Err(Error::Rejected { kind }) if kind == "overQuota"
        ));
    }

    #[test]
    fn sieve_scripts_are_listed() {
        let http = ScriptedHttp::new().json(SESSION).json(
            r#"{"methodResponses":[["SieveScript/get",{"accountId":"acc-1","state":"sv-1","notFound":[],"list":[
                {"id":"s1","name":"vacation","blobId":"b1","isActive":false},
                {"id":"s2","name":null,"blobId":"b2","isActive":true}]},"c0"]]}"#,
        );
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let scripts = poll_now(client.sieve_scripts()).unwrap().unwrap();
        let active: Vec<&str> = scripts
            .iter()
            .filter(|s| s.is_active)
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(scripts.len(), 2);
        assert_eq!(active, ["s2"]);
        assert_eq!(scripts[0].name.as_deref(), Some("vacation"));
        let sent: Value = serde_json::from_slice(&http.requests()[1].body).unwrap();
        assert_eq!(sent["using"][1], "urn:ietf:params:jmap:sieve");
    }
}
