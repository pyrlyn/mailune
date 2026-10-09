//! A script of [`Submission`]s folded into [`Event`]s on a [`FakeHost`].
//!
//! The mailbox exists only for the fold. A send posts a banner on the host so
//! the script is not a pure data transform.

use mailune_protocol::{
    AccountId, Address, BackgroundScheduler, BackgroundWork, Category, Error, Event, MailboxId,
    NetworkPath, NetworkState, Notification, Notifier, Submission, ThreadId, ThreadRow,
};

use crate::FakeHost;

/// Ordered submissions. [`Scenario::fold`] replays them from an empty mailbox.
#[derive(Debug, Default)]
pub struct Scenario {
    steps: Vec<Submission>,
}

impl Scenario {
    /// An empty script.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one submission. The script is not run yet.
    pub fn then(mut self, submission: Submission) -> Self {
        self.steps.push(submission);
        self
    }

    /// Applies every submission and returns the events, in order.
    ///
    /// # Errors
    ///
    /// [`Error`] when the host refuses a call the step needed. A missing
    /// thread is a notice, so one bad id does not drop the rest of the script.
    pub fn fold(&self, host: &FakeHost) -> Result<Vec<Event>, Error> {
        let mut mail = Mail::default();
        let mut events = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            events.push(apply(&mut mail, host, step)?);
        }
        Ok(events)
    }

    /// The same fold, as JSON another front end can replay.
    ///
    /// `steps` are the submissions. `events` are what [`Scenario::fold`]
    /// returned for this host. The fold itself is unchanged.
    ///
    /// # Errors
    ///
    /// The same [`Error`] as [`Scenario::fold`]. Encoding the value is a host
    /// error; these types already serialize.
    pub fn fold_json(&self, host: &FakeHost) -> Result<String, Error> {
        let events = self.fold(host)?;
        let value = serde_json::json!({
            "steps": &self.steps,
            "events": &events,
        });
        serde_json::to_string(&value)
            .map_err(|err| Error::host("scenario.fold_json", err.to_string()))
    }
}

struct Mail {
    account: AccountId,
    threads: Vec<ThreadRow>,
    past: Vec<Vec<ThreadRow>>,
    seq: u64,
}

impl Default for Mail {
    fn default() -> Self {
        Self {
            account: AccountId::new("local"),
            threads: Vec::new(),
            past: Vec::new(),
            seq: 0,
        }
    }
}

fn apply(mail: &mut Mail, host: &FakeHost, step: &Submission) -> Result<Event, Error> {
    Ok(match step {
        Submission::Refresh => snap(host, &mail.threads),
        Submission::OpenThread { thread } => open(mail, thread),
        Submission::Send { to, subject, body } => send(mail, host, to, subject, body)?,
        Submission::SaveDraft { subject, .. } => note(format!("draft {subject}")),
        Submission::Archive { threads } => place(mail, host, threads, "archive"),
        Submission::Delete { threads } => place(mail, host, threads, "trash"),
        Submission::Spam { threads } => place(mail, host, threads, "spam"),
        Submission::Snooze { thread, .. } => flip(mail, host, thread, |row| row.snoozed = true),
        Submission::MoveTo { thread, mailbox } => {
            place(mail, host, std::slice::from_ref(thread), mailbox.as_str())
        }
        Submission::ToggleStar { thread } => {
            flip(mail, host, thread, |row| row.flagged = !row.flagged)
        }
        Submission::ToggleRead { thread } => {
            flip(mail, host, thread, |row| row.unread = !row.unread)
        }
        Submission::ApplyLabel { thread, label } => {
            let label = label.clone();
            flip(mail, host, thread, move |row| toggle_label(row, &label))
        }
        Submission::Search { query } => search(mail, query),
        Submission::SwitchAccount { account } => {
            mail.account = account.clone();
            host.schedule(account, BackgroundWork::Sync)?;
            snap(host, &mail.threads)
        }
        Submission::Undo => undo(mail, host),
        Submission::Download { name } => note(format!("download {name}")),
        Submission::SignIn { email } => note(format!("sign-in {email}")),
        Submission::SignUp { email, .. } => note(format!("sign-up {email}")),
        Submission::SignOut => {
            host.cancel(&mail.account, BackgroundWork::Sync)?;
            note("signed out".into())
        }
        Submission::ChoosePlan { plan } => note(format!("plan {plan}")),
        Submission::SetLanguage { code } => note(format!(
            "language {}",
            if code.is_empty() { "en" } else { code }
        )),
        Submission::Summarize { .. } => note("summarize".into()),
    })
}

fn send(
    mail: &mut Mail,
    host: &FakeHost,
    to: &[Address],
    subject: &str,
    body: &str,
) -> Result<Event, Error> {
    remember(mail);
    mail.seq += 1;
    let who = to
        .first()
        .map(|item| item.email.as_str())
        .unwrap_or("undisclosed");
    let line = body.lines().next().unwrap_or("");
    mail.threads.push(row(
        &mail.account,
        &format!("t{}", mail.seq),
        subject,
        &format!("to {who}: {line}"),
        "sent",
    ));
    host.notify(&Notification {
        account: mail.account.clone(),
        title: "Sent".into(),
        body: subject.to_string(),
    });
    if host.path() == NetworkPath::Offline {
        host.schedule(&mail.account, BackgroundWork::Sync)?;
    }
    Ok(snap(host, &mail.threads))
}

fn place(mail: &mut Mail, host: &FakeHost, ids: &[ThreadId], mailbox: &str) -> Event {
    if !ids.iter().any(|id| has(mail, id)) {
        return missing();
    }
    remember(mail);
    for item in &mut mail.threads {
        if ids.contains(&item.id) {
            item.mailbox = MailboxId::new(mailbox);
        }
    }
    snap(host, &mail.threads)
}

fn flip(
    mail: &mut Mail,
    host: &FakeHost,
    id: &ThreadId,
    change: impl FnOnce(&mut ThreadRow),
) -> Event {
    if !has(mail, id) {
        return missing();
    }
    remember(mail);
    if let Some(item) = mail.threads.iter_mut().find(|item| &item.id == id) {
        change(item);
    }
    snap(host, &mail.threads)
}

fn open(mail: &Mail, id: &ThreadId) -> Event {
    match mail.threads.iter().find(|item| &item.id == id) {
        Some(item) => Event::Snapshot {
            threads: vec![item.clone()],
        },
        None => missing(),
    }
}

fn search(mail: &Mail, query: &str) -> Event {
    let needle = query.to_ascii_lowercase();
    Event::Snapshot {
        threads: mail
            .threads
            .iter()
            .filter(|item| item.subject.to_ascii_lowercase().contains(&needle))
            .cloned()
            .collect(),
    }
}

fn undo(mail: &mut Mail, host: &FakeHost) -> Event {
    match mail.past.pop() {
        Some(previous) => {
            mail.threads = previous;
            snap(host, &mail.threads)
        }
        None => note("nothing to undo".into()),
    }
}

fn remember(mail: &mut Mail) {
    mail.past.push(mail.threads.clone());
}

fn has(mail: &Mail, id: &ThreadId) -> bool {
    mail.threads.iter().any(|item| &item.id == id)
}

fn toggle_label(row: &mut ThreadRow, label: &str) {
    if let Some(index) = row.labels.iter().position(|item| item == label) {
        row.labels.remove(index);
    } else {
        row.labels.push(label.to_string());
    }
}

fn snap(host: &FakeHost, threads: &[ThreadRow]) -> Event {
    let unread = u32::try_from(threads.iter().filter(|row| row.unread).count()).unwrap_or(u32::MAX);
    host.set_badge(unread);
    Event::Snapshot {
        threads: threads.to_vec(),
    }
}

fn note(message: String) -> Event {
    Event::Notice { message }
}

fn missing() -> Event {
    note("missing thread".into())
}

fn row(account: &AccountId, id: &str, subject: &str, snippet: &str, mailbox: &str) -> ThreadRow {
    ThreadRow {
        id: ThreadId::new(id),
        account: account.clone(),
        from: Address {
            name: None,
            email: "me@mailune.local".into(),
        },
        subject: subject.to_string(),
        snippet: snippet.to_string(),
        stamp: "t0".into(),
        message_count: 1,
        unread: false,
        flagged: false,
        important: false,
        pinned: false,
        snoozed: false,
        draft: false,
        has_attachment: false,
        category: Category::Primary,
        mailbox: MailboxId::new(mailbox),
        labels: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{
        AccountId, Address, BackgroundWork, Event, MailboxId, NetworkPath, Submission, ThreadId,
    };

    use super::Scenario;
    use crate::FakeHost;

    fn sent() -> Submission {
        Submission::Send {
            to: vec![Address {
                name: None,
                email: "ada@example.com".into(),
            }],
            subject: "Hello".into(),
            body: "plain body".into(),
        }
    }

    #[test]
    fn send_then_archive_folds_into_snapshots() {
        let host = FakeHost::new();
        host.set_path(NetworkPath::Unmetered);
        let events = Scenario::new()
            .then(sent())
            .then(Submission::Archive {
                threads: vec![ThreadId::new("t1")],
            })
            .fold(&host)
            .unwrap();
        let Event::Snapshot { threads } = &events[0] else {
            panic!("send");
        };
        assert_eq!(threads[0].mailbox, MailboxId::new("sent"));
        assert!(threads[0].snippet.contains("ada@example.com"));
        let Event::Snapshot { threads } = &events[1] else {
            panic!("archive");
        };
        assert_eq!(threads[0].mailbox, MailboxId::new("archive"));
        assert_eq!(host.notices().len(), 1);
        assert!(host.jobs().is_empty());
    }

    #[test]
    fn an_offline_send_schedules_sync_and_undo_restores() {
        let host = FakeHost::new();
        let events = Scenario::new()
            .then(sent())
            .then(Submission::Undo)
            .fold(&host)
            .unwrap();
        assert_eq!(
            host.jobs(),
            [(AccountId::new("local"), BackgroundWork::Sync)]
        );
        let Event::Snapshot { threads } = &events[1] else {
            panic!("undo");
        };
        assert!(threads.is_empty());
    }

    #[test]
    fn fold_json_is_the_same_events_another_front_end_can_read() {
        let host = FakeHost::new();
        let scenario = Scenario::new().then(sent()).then(Submission::Undo);
        let events = scenario.fold(&host).unwrap();
        let host = FakeHost::new();
        let text = scenario.fold_json(&host).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let again: Vec<Event> = serde_json::from_value(value["events"].clone()).unwrap();
        let steps: Vec<Submission> = serde_json::from_value(value["steps"].clone()).unwrap();
        assert_eq!(again, events);
        assert_eq!(steps.len(), 2);
        assert!(matches!(steps[0], Submission::Send { .. }));
    }

    #[test]
    fn macos_thread_fixture_is_a_contract_snapshot() {
        let text = include_str!(
            "../../../desktop/macos/MailuneModel/Sources/MailuneModel/Fixtures/threads.json"
        );
        let Event::Snapshot { threads } = serde_json::from_str(text).unwrap() else {
            panic!("the macOS fixture is not a Snapshot event");
        };
        assert_eq!(threads.len(), 4);
        assert!(threads.iter().any(|row| row.has_attachment));
        assert!(threads.iter().any(|row| row.from.name.is_none()));
    }
}
