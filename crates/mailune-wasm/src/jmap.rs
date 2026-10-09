//! JMAP calls for the browser, run over a transport the caller supplies.

use mailune_jmap::JmapClient;
use mailune_protocol::Secret;
use serde_json::Value;

use crate::replay::{Replay, run};

pub(crate) fn mailbox_ids(session_url: &str, replies: Vec<String>) -> Result<String, String> {
    let http = Replay::new(replies);
    // Recorded replies need no credential; the fetch transport will carry
    // the real one.
    let mut client = JmapClient::new(&http, Secret::new(""));
    run(client.load_session(session_url))?.map_err(|error| error.to_string())?;
    let (mailboxes, _state) = run(client.mailboxes())?.map_err(|error| error.to_string())?;
    let ids = mailboxes
        .into_iter()
        .map(|mailbox| Value::String(mailbox.id.as_str().to_string()))
        .collect();
    Ok(Value::Array(ids).to_string())
}

#[cfg(test)]
mod tests {
    use mailune_jmap::JmapClient;
    use mailune_protocol::Secret;
    use mailune_testkit::{ScriptedHttp, poll_now};

    use crate::jmap_mailbox_ids;

    const SESSION: &str = include_str!("../../mailune-jmap/fixtures/session.json");
    const MAILBOXES: &str = include_str!("../../mailune-jmap/fixtures/sync-initial.json");
    const URL: &str = "https://jmap.example.com/.well-known/jmap";

    #[test]
    fn the_web_query_returns_the_native_mailbox_ids() {
        let http = ScriptedHttp::new().json(SESSION).json(MAILBOXES);
        let mut client = JmapClient::new(&http, Secret::new("t"));
        poll_now(client.load_session(URL)).unwrap().unwrap();
        let (native, _) = poll_now(client.mailboxes()).unwrap().unwrap();
        let native: Vec<&str> = native.iter().map(|m| m.id.as_str()).collect();
        assert!(!native.is_empty());

        let web = jmap_mailbox_ids(URL, vec![SESSION.into(), MAILBOXES.into()]).unwrap();
        let web: Vec<String> = serde_json::from_str(&web).unwrap();
        assert_eq!(web, native);
    }

    #[test]
    fn a_missing_reply_is_an_error() {
        assert!(jmap_mailbox_ids(URL, vec![SESSION.into()]).is_err());
        assert!(jmap_mailbox_ids(URL, Vec::new()).is_err());
    }
}
