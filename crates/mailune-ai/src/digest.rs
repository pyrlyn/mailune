//! Daily digest: one short summary per thread that has mail since an instant.
//!
//! Only messages received at or after `since` reach the model, so yesterday's
//! mail neither costs a call nor leaks into today's digest. Each entry cites
//! the message ids it was built from.

use std::time::SystemTime;

use mailune_protocol::{MessageId, ThreadId};

use crate::{Error, MailText, Privacy, Provider, SummaryCache, SummaryKind};

/// One thread offered to the digest.
#[derive(Debug, Clone)]
pub struct DigestThread {
    /// Which conversation.
    pub thread: ThreadId,
    /// Where its prompt may run.
    pub privacy: Privacy,
    /// Its messages, any order.
    pub messages: Vec<MailText>,
}

/// One line of the digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestEntry {
    /// Which conversation.
    pub thread: ThreadId,
    /// Short summary of the new messages.
    pub summary: String,
    /// Messages the summary was built from.
    pub cites: Vec<MessageId>,
    /// Newest of those messages.
    pub latest: SystemTime,
}

/// The digest, newest thread first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest {
    /// Start of the window.
    pub since: SystemTime,
    /// Threads with new mail.
    pub entries: Vec<DigestEntry>,
}

/// Builds a digest of `threads` covering mail received at or after `since`.
///
/// # Errors
///
/// The provider's error for the first thread that fails.
pub async fn daily_digest<P: Provider>(
    cache: &mut SummaryCache,
    provider: &P,
    threads: &[DigestThread],
    since: SystemTime,
) -> Result<Digest, Error> {
    let mut entries = Vec::new();
    for thread in threads {
        let mut fresh: Vec<MailText> = thread
            .messages
            .iter()
            .filter(|message| message.received >= since)
            .cloned()
            .collect();
        let Some(latest) = fresh.iter().map(|message| message.received).max() else {
            continue;
        };
        fresh.sort_by_key(|message| message.received);
        let summary = cache
            .summarize(provider, &fresh, SummaryKind::Short, thread.privacy)
            .await?;
        entries.push(DigestEntry {
            thread: thread.thread.clone(),
            summary,
            cites: fresh.into_iter().map(|message| message.id).collect(),
            latest,
        });
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.latest));
    Ok(Digest { since, entries })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use mailune_protocol::{MessageId, ThreadId};

    use super::{DigestThread, daily_digest};
    use crate::summary::tests::{LOCAL, mail};
    use crate::testing::drive;
    use crate::{Feature, LocalProvider, ScriptedEngine, SummaryCache};

    #[test]
    fn the_digest_covers_mail_since_the_instant_and_skips_older() {
        let threads = vec![
            DigestThread {
                thread: ThreadId::new("old"),
                privacy: LOCAL,
                messages: vec![mail("m0", 5, "Last week's lunch")],
            },
            DigestThread {
                thread: ThreadId::new("launch"),
                privacy: LOCAL,
                messages: vec![
                    mail("m2", 130, "QA signed off"),
                    mail("m1", 50, "Old launch note"),
                ],
            },
            DigestThread {
                thread: ThreadId::new("invoice"),
                privacy: LOCAL,
                messages: vec![mail("m3", 200, "Invoice 42 is due")],
            },
        ];
        let engine = ScriptedEngine::new(["QA is done.", "Invoice due."], [Feature::Summarize]);
        let provider = LocalProvider::new(engine, 128);
        let since = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 60);
        let mut cache = SummaryCache::new();
        let digest = drive(daily_digest(&mut cache, &provider, &threads, since)).unwrap();

        assert_eq!(digest.since, since);
        let ids: Vec<&str> = digest.entries.iter().map(|e| e.thread.as_str()).collect();
        assert_eq!(ids, ["invoice", "launch"]);
        assert_eq!(digest.entries[1].summary, "QA is done.");
        assert_eq!(digest.entries[1].cites, [MessageId::new("m2")]);
        let prompts = provider.engine().seen();
        assert_eq!(prompts.len(), 2);
        assert!(
            prompts
                .iter()
                .all(|p| !p.prompt.contains("Old launch note"))
        );
        assert!(prompts.iter().all(|p| !p.prompt.contains("lunch")));

        // A second digest over the same window is served from the cache.
        let again = drive(daily_digest(&mut cache, &provider, &threads, since)).unwrap();
        assert_eq!(again, digest);
        assert_eq!(provider.engine().seen().len(), 2);
    }
}
