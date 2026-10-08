//! Mutations: keyword changes and moves through `Email/set`, and sending
//! through `EmailSubmission/set`. Each change is a patch on named paths, so
//! replaying it is idempotent.

use std::collections::BTreeMap;

use mailune_protocol::{Http, MailboxId, MessageId};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::wire::{USING, json_error, take};
use crate::{Error, JmapClient};

const SUBMISSION: &str = "urn:ietf:params:jmap:submission";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetResult {
    #[serde(default)]
    created: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    not_created: Option<BTreeMap<String, SetError>>,
    #[serde(default)]
    not_updated: Option<BTreeMap<String, SetError>>,
}

#[derive(Deserialize)]
struct SetError {
    #[serde(rename = "type")]
    kind: String,
}

impl<H: Http> JmapClient<'_, H> {
    /// Sets (`true`) or clears (`false`) keywords on one email, such as
    /// `$seen` or `$flagged`. Other keywords are left alone.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] when the server refuses the update.
    pub async fn set_keywords(
        &self,
        email: &MessageId,
        changes: &[(&str, bool)],
    ) -> Result<(), Error> {
        let mut patch = Map::new();
        for (keyword, on) in changes {
            // `null` removes the key: JMAP has no `false` keyword value.
            let value = if *on { Value::Bool(true) } else { Value::Null };
            patch.insert(format!("keywords/{keyword}"), value);
        }
        self.update_email(email, patch).await
    }

    /// Moves one email from `from` to `to`, leaving any other mailbox it is in.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] when the server refuses the update.
    pub async fn move_email(
        &self,
        email: &MessageId,
        from: &MailboxId,
        to: &MailboxId,
    ) -> Result<(), Error> {
        let mut patch = Map::new();
        patch.insert(format!("mailboxIds/{}", from.as_str()), Value::Null);
        patch.insert(format!("mailboxIds/{}", to.as_str()), Value::Bool(true));
        self.update_email(email, patch).await
    }

    /// Submits a saved draft for delivery with `identity`, and on success
    /// clears its `$draft` keyword and files it in `sent`. Call this only
    /// after the person confirmed the send in the app.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] when the server refuses the submission.
    pub async fn submit(
        &self,
        email: &MessageId,
        identity: &str,
        drafts: &MailboxId,
        sent: &MailboxId,
    ) -> Result<String, Error> {
        let account = self.account()?;
        let on_success = json!({
            "#send": {
                "keywords/$draft": null,
                format!("mailboxIds/{}", drafts.as_str()): null,
                format!("mailboxIds/{}", sent.as_str()): true,
            }
        });
        let responses = self
            .call_using(
                &[USING[0], USING[1], SUBMISSION],
                &[(
                    "EmailSubmission/set",
                    json!({
                        "accountId": account,
                        "create": { "send": { "identityId": identity, "emailId": email } },
                        "onSuccessUpdateEmail": on_success,
                    }),
                )],
            )
            .await?;
        let result: SetResult = take(&responses, "EmailSubmission/set")?;
        if let Some(error) = result.not_created.and_then(|mut map| map.remove("send")) {
            return Err(Error::Rejected { kind: error.kind });
        }
        result
            .created
            .and_then(|mut map| map.remove("send"))
            .and_then(|created| {
                created
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .ok_or_else(|| Error::Json("EmailSubmission/set created no id".into()))
    }

    async fn update_email(
        &self,
        email: &MessageId,
        patch: Map<String, Value>,
    ) -> Result<(), Error> {
        let account = self.account()?;
        let mut update = Map::new();
        update.insert(email.as_str().to_string(), Value::Object(patch));
        let responses = self
            .call(&[(
                "Email/set",
                json!({ "accountId": account, "update": update }),
            )])
            .await?;
        let (_, args) = responses
            .iter()
            .find(|(name, _)| name == "Email/set")
            .ok_or_else(|| Error::Json("missing Email/set response".into()))?;
        let result = SetResult::deserialize(args).map_err(json_error)?;
        if let Some(error) = result
            .not_updated
            .and_then(|mut map| map.remove(email.as_str()))
        {
            return Err(Error::Rejected { kind: error.kind });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{MailboxId, MessageId, Secret};
    use mailune_testkit::{ScriptedHttp, poll_now};
    use serde_json::Value;

    use crate::{Error, JmapClient};

    const SESSION: &str = include_str!("../fixtures/session.json");

    fn body(http: &ScriptedHttp, index: usize) -> Value {
        serde_json::from_slice(&http.requests()[index].body).unwrap()
    }

    #[test]
    fn a_flag_change_is_a_keyword_patch() {
        let http = ScriptedHttp::new().json(SESSION).json(
            r#"{"methodResponses":[["Email/set",{"accountId":"acc-1","oldState":"e-1","newState":"e-2","updated":{"m1":null}},"c0"]]}"#,
        ).json(
            r#"{"methodResponses":[["Email/set",{"accountId":"acc-1","newState":"e-3","notUpdated":{"m1":{"type":"notFound"}}},"c0"]]}"#,
        );
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let id = MessageId::new("m1");
        poll_now(client.set_keywords(&id, &[("$seen", true), ("$flagged", false)]))
            .unwrap()
            .unwrap();
        let sent = body(&http, 1);
        let call = &sent["methodCalls"][0];
        assert_eq!(call[0], "Email/set");
        assert_eq!(call[1]["update"]["m1"]["keywords/$seen"], true);
        assert!(call[1]["update"]["m1"]["keywords/$flagged"].is_null());
        assert!(matches!(
            poll_now(client.move_email(&id, &MailboxId::new("a"), &MailboxId::new("b"))).unwrap(),
            Err(Error::Rejected { kind }) if kind == "notFound"
        ));
        let moved = body(&http, 2);
        assert!(moved["methodCalls"][0][1]["update"]["m1"]["mailboxIds/a"].is_null());
        assert_eq!(
            moved["methodCalls"][0][1]["update"]["m1"]["mailboxIds/b"],
            true
        );
    }

    #[test]
    fn a_submission_sends_the_draft_and_files_it() {
        let http = ScriptedHttp::new().json(SESSION).json(
            r#"{"methodResponses":[
                ["EmailSubmission/set",{"accountId":"acc-1","newState":"s2","created":{"send":{"id":"sub-9","undoStatus":"pending"}}},"c0"],
                ["Email/set",{"accountId":"acc-1","newState":"e-4","updated":{"m7":null}},"c0"]
            ]}"#,
        ).json(
            r#"{"methodResponses":[["EmailSubmission/set",{"accountId":"acc-1","newState":"s3","notCreated":{"send":{"type":"forbiddenFrom"}}},"c0"]]}"#,
        );
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let (drafts, sent) = (MailboxId::new("mb-drafts"), MailboxId::new("mb-sent"));
        let id = poll_now(client.submit(&MessageId::new("m7"), "id-1", &drafts, &sent))
            .unwrap()
            .unwrap();
        assert_eq!(id, "sub-9");
        let request = body(&http, 1);
        assert_eq!(request["using"][2], "urn:ietf:params:jmap:submission");
        let args = &request["methodCalls"][0][1];
        assert_eq!(args["create"]["send"]["emailId"], "m7");
        assert!(args["onSuccessUpdateEmail"]["#send"]["keywords/$draft"].is_null());
        assert_eq!(
            args["onSuccessUpdateEmail"]["#send"]["mailboxIds/mb-sent"],
            true
        );
        assert!(matches!(
            poll_now(client.submit(&MessageId::new("m8"), "id-1", &drafts, &sent)).unwrap(),
            Err(Error::Rejected { kind }) if kind == "forbiddenFrom"
        ));
    }
}
