//! Agent tool calls with a scope, a preview, an undo record and an audit line.
//!
//! The model proposes a [`ToolCall`]. [`Agent::propose`] checks it against the
//! [`Policy`] and the [`Scope`] the person granted and returns a [`Pending`]
//! call the app can show. Nothing is applied here: [`Agent::commit`] returns
//! the submissions for the app to run, and send and delete stay refused until
//! the app passes [`Confirmation::Confirmed`]. Every decision, including a
//! denial, appends one audit line that names no mail content.

use std::fmt;

use mailune_protocol::{Address, MailboxId, Submission, ThreadId};
use serde::Deserialize;

use crate::guard::tool_name;
use crate::{Error, Policy, Tool, ToolProposal};

/// A typed call the model may propose.
///
/// Forward has no submission in the contract yet, so it is not a variant: a
/// model that asks for it fails to parse and is denied.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolCall {
    /// Summarize one conversation. Changes nothing.
    Summarize {
        /// Which conversation.
        thread: ThreadId,
    },
    /// Archive conversations.
    Archive {
        /// Conversations to archive.
        threads: Vec<ThreadId>,
    },
    /// Move conversations to Trash. Needs confirmation.
    Delete {
        /// Conversations to delete.
        threads: Vec<ThreadId>,
    },
    /// Send a new message. Needs confirmation.
    Send {
        /// Recipients.
        to: Vec<Address>,
        /// Subject line.
        subject: String,
        /// Plain body.
        body: String,
    },
}

impl ToolCall {
    /// The permission this call needs.
    pub fn tool(&self) -> Tool {
        match self {
            Self::Summarize { .. } => Tool::Summarize,
            Self::Archive { .. } => Tool::Archive,
            Self::Delete { .. } => Tool::Delete,
            Self::Send { .. } => Tool::Send,
        }
    }

    /// Conversations the call touches. Send touches none.
    pub fn threads(&self) -> &[ThreadId] {
        match self {
            Self::Summarize { thread } => std::slice::from_ref(thread),
            Self::Archive { threads } | Self::Delete { threads } => threads,
            Self::Send { .. } => &[],
        }
    }
}

/// Parses one model reply as a [`ToolCall`]. The whole reply must be one JSON
/// object with no unknown field.
///
/// # Errors
///
/// [`Error::ToolDenied`] for anything else.
pub fn parse_call(output: &str) -> Result<ToolCall, Error> {
    serde_json::from_str(output.trim()).map_err(|_| Error::ToolDenied)
}

/// Whether the app must ask the person before this tool runs.
pub fn needs_confirmation(tool: Tool) -> bool {
    matches!(tool, Tool::Send | Tool::Delete | Tool::Forward)
}

/// The conversations the person put in front of the agent, with the mailbox
/// each one is in now. An undo moves a conversation back there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    threads: Vec<(ThreadId, MailboxId)>,
}

impl Scope {
    /// A scope over exactly these conversations.
    pub fn new(threads: impl IntoIterator<Item = (ThreadId, MailboxId)>) -> Self {
        Self {
            threads: threads.into_iter().collect(),
        }
    }

    /// The mailbox `thread` is in, when the scope has it.
    pub fn mailbox(&self, thread: &ThreadId) -> Option<&MailboxId> {
        self.threads
            .iter()
            .find(|(id, _)| id == thread)
            .map(|(_, mailbox)| mailbox)
    }
}

/// The answer the app got from the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// The person has not confirmed.
    NotAsked,
    /// The person confirmed this call in the app.
    Confirmed,
}

/// What the app shows before the call runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// The tool.
    pub tool: Tool,
    /// One English line for the confirmation sheet.
    pub text: String,
    /// Conversations touched.
    pub threads: usize,
    /// Recipient addresses for send.
    pub recipients: Vec<String>,
    /// The app must confirm before [`Agent::commit`].
    pub needs_confirmation: bool,
}

/// How to reverse one committed call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoRecord {
    /// Call this record reverses.
    pub call: u64,
    /// Tool of that call, for the audit line.
    pub tool: Tool,
    /// Submissions that put things back. Empty when nothing changed.
    pub inverse: Vec<Submission>,
    /// False for send: a sent message cannot be called back here. The
    /// queue's undo-send delay is the only window.
    pub reversible: bool,
}

/// A checked call that has not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// Sequence number, also on the audit lines.
    pub id: u64,
    /// The call.
    pub call: ToolCall,
    /// What the app shows.
    pub preview: Preview,
    /// How to reverse it once committed.
    pub undo: UndoRecord,
}

/// What happened to a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Policy, scope or parse refused it.
    Denied,
    /// Checked and shown as a preview.
    Proposed,
    /// Refused at commit because the app had not confirmed.
    Unconfirmed,
    /// Handed to the app to apply.
    Committed,
    /// Reversed.
    Undone,
}

/// One audit row. It names the call, the tool and a count, never mail text
/// or an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditLine {
    /// Call sequence number; 0 for a reply that never became a call.
    pub call: u64,
    /// The tool, when the reply parsed.
    pub tool: Option<Tool>,
    /// Conversations touched.
    pub threads: usize,
    /// Decision.
    pub outcome: Outcome,
}

impl fmt::Display for AuditLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tool = self.tool.map_or("unknown", tool_name);
        let outcome = match self.outcome {
            Outcome::Denied => "denied",
            Outcome::Proposed => "proposed",
            Outcome::Unconfirmed => "unconfirmed",
            Outcome::Committed => "committed",
            Outcome::Undone => "undone",
        };
        write!(
            formatter,
            "call={} tool={tool} threads={} outcome={outcome}",
            self.call, self.threads
        )
    }
}

/// Runs the checks for one agent session.
#[derive(Debug, Clone)]
pub struct Agent {
    policy: Policy,
    scope: Scope,
    next: u64,
    audit: Vec<AuditLine>,
    undo: Vec<UndoRecord>,
    held: Vec<Pending>,
}

/// What a request from a caller that cannot confirm became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requested {
    /// Ran: the app applies these submissions.
    Applied(Vec<Submission>),
    /// Waits for the person to approve it in the app.
    Held {
        /// Id to pass to [`Agent::approve`].
        call: u64,
        /// What the app shows.
        preview: Preview,
    },
}

impl Agent {
    /// An agent bound to `policy` and `scope`.
    pub fn new(policy: Policy, scope: Scope) -> Self {
        Self {
            policy,
            scope,
            next: 0,
            audit: Vec::new(),
            undo: Vec::new(),
            held: Vec::new(),
        }
    }

    /// Audit lines, oldest first.
    pub fn audit(&self) -> &[AuditLine] {
        &self.audit
    }

    /// Parses a model reply and proposes it.
    ///
    /// # Errors
    ///
    /// As [`parse_call`] and [`Agent::propose`]. A reply that does not parse
    /// still leaves an audit line.
    pub fn propose_reply(&mut self, output: &str) -> Result<Pending, Error> {
        match parse_call(output) {
            Ok(call) => self.propose(call),
            Err(err) => {
                self.log(0, None, 0, Outcome::Denied);
                Err(err)
            }
        }
    }

    /// Checks `call` and builds its preview and undo record.
    ///
    /// # Errors
    ///
    /// [`Error::ToolDenied`] when the policy does not list the tool or the
    /// call names no conversation it needs. [`Error::OutOfScope`] when it
    /// names a conversation outside the scope.
    pub fn propose(&mut self, call: ToolCall) -> Result<Pending, Error> {
        self.next += 1;
        let id = self.next;
        let tool = call.tool();
        let threads = call.threads().len();
        if let Err(err) = self.check(&call) {
            self.log(id, Some(tool), threads, Outcome::Denied);
            return Err(err);
        }
        let pending = Pending {
            id,
            preview: preview(&call),
            undo: self.undo_record(id, &call),
            call,
        };
        self.log(id, Some(tool), threads, Outcome::Proposed);
        Ok(pending)
    }

    /// Hands `pending` to the app as submissions and keeps its undo record.
    ///
    /// # Errors
    ///
    /// [`Error::NeedsConfirmation`] when the tool needs the app's
    /// confirmation and `confirmation` is not [`Confirmation::Confirmed`].
    pub fn commit(
        &mut self,
        pending: Pending,
        confirmation: Confirmation,
    ) -> Result<Vec<Submission>, Error> {
        let tool = pending.call.tool();
        let threads = pending.call.threads().len();
        if pending.preview.needs_confirmation && confirmation != Confirmation::Confirmed {
            self.log(pending.id, Some(tool), threads, Outcome::Unconfirmed);
            return Err(Error::NeedsConfirmation);
        }
        self.log(pending.id, Some(tool), threads, Outcome::Committed);
        if !pending.undo.inverse.is_empty() {
            self.undo.push(pending.undo);
        }
        Ok(submissions(pending.call))
    }

    /// Proposes `call` for a caller that cannot confirm, such as an MCP
    /// client. A call that needs confirmation is held for the app, never run.
    ///
    /// # Errors
    ///
    /// As [`Agent::propose`].
    pub fn request(&mut self, call: ToolCall) -> Result<Requested, Error> {
        let pending = self.propose(call)?;
        if pending.preview.needs_confirmation {
            let held = Requested::Held {
                call: pending.id,
                preview: pending.preview.clone(),
            };
            self.held.push(pending);
            return Ok(held);
        }
        self.commit(pending, Confirmation::NotAsked)
            .map(Requested::Applied)
    }

    /// Calls waiting for the app, oldest first.
    pub fn held(&self) -> &[Pending] {
        &self.held
    }

    /// The app's answer for held call `id`. An unconfirmed answer leaves it held.
    ///
    /// # Errors
    ///
    /// [`Error::ToolDenied`] when no call with that id is held, and
    /// [`Error::NeedsConfirmation`] when `confirmation` is not
    /// [`Confirmation::Confirmed`].
    pub fn approve(
        &mut self,
        id: u64,
        confirmation: Confirmation,
    ) -> Result<Vec<Submission>, Error> {
        let index = self
            .held
            .iter()
            .position(|pending| pending.id == id)
            .ok_or(Error::ToolDenied)?;
        if confirmation != Confirmation::Confirmed {
            let call = &self.held[index].call;
            let (tool, threads) = (call.tool(), call.threads().len());
            self.log(id, Some(tool), threads, Outcome::Unconfirmed);
            return Err(Error::NeedsConfirmation);
        }
        let pending = self.held.remove(index);
        self.commit(pending, confirmation)
    }

    /// Reverses the newest committed call that changed something.
    pub fn undo(&mut self) -> Option<Vec<Submission>> {
        let record = self.undo.pop()?;
        self.log(
            record.call,
            Some(record.tool),
            record.inverse.len(),
            Outcome::Undone,
        );
        Some(record.inverse)
    }

    fn check(&self, call: &ToolCall) -> Result<(), Error> {
        self.policy.approve(&ToolProposal { tool: call.tool() })?;
        let threads = call.threads();
        if threads.is_empty() && call.tool() != Tool::Send {
            // A bulk tool with no target is not a no-op the model may get for free.
            return Err(Error::ToolDenied);
        }
        if threads
            .iter()
            .any(|thread| self.scope.mailbox(thread).is_none())
        {
            return Err(Error::OutOfScope);
        }
        Ok(())
    }

    fn undo_record(&self, id: u64, call: &ToolCall) -> UndoRecord {
        let inverse = match call {
            ToolCall::Summarize { .. } | ToolCall::Send { .. } => Vec::new(),
            ToolCall::Archive { threads } | ToolCall::Delete { threads } => threads
                .iter()
                .filter_map(|thread| {
                    self.scope
                        .mailbox(thread)
                        .map(|mailbox| Submission::MoveTo {
                            thread: thread.clone(),
                            mailbox: mailbox.clone(),
                        })
                })
                .collect(),
        };
        UndoRecord {
            call: id,
            tool: call.tool(),
            inverse,
            reversible: !matches!(call, ToolCall::Send { .. }),
        }
    }

    fn log(&mut self, call: u64, tool: Option<Tool>, threads: usize, outcome: Outcome) {
        self.audit.push(AuditLine {
            call,
            tool,
            threads,
            outcome,
        });
    }
}

fn preview(call: &ToolCall) -> Preview {
    let threads = call.threads().len();
    let (text, recipients) = match call {
        ToolCall::Summarize { .. } => ("Summarize 1 conversation".to_string(), Vec::new()),
        ToolCall::Archive { .. } => (format!("Archive {threads} conversation(s)"), Vec::new()),
        ToolCall::Delete { .. } => (
            format!("Move {threads} conversation(s) to Trash"),
            Vec::new(),
        ),
        ToolCall::Send { to, subject, .. } => (
            format!("Send \"{subject}\" to {} recipient(s)", to.len()),
            to.iter().map(|address| address.email.clone()).collect(),
        ),
    };
    Preview {
        tool: call.tool(),
        text,
        threads,
        recipients,
        needs_confirmation: needs_confirmation(call.tool()),
    }
}

fn submissions(call: ToolCall) -> Vec<Submission> {
    match call {
        ToolCall::Summarize { thread } => vec![Submission::Summarize { thread }],
        ToolCall::Archive { threads } => vec![Submission::Archive { threads }],
        ToolCall::Delete { threads } => vec![Submission::Delete { threads }],
        ToolCall::Send { to, subject, body } => vec![Submission::Send { to, subject, body }],
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{MailboxId, Submission, ThreadId};

    use super::{Agent, Confirmation, Outcome, Requested, Scope, ToolCall, parse_call};
    use crate::{Error, Policy, Tool};

    fn scope() -> Scope {
        Scope::new([
            (ThreadId::new("t1"), MailboxId::new("inbox")),
            (ThreadId::new("t2"), MailboxId::new("work")),
        ])
    }

    fn agent(tools: impl IntoIterator<Item = Tool>) -> Agent {
        Agent::new(Policy::new(tools), scope())
    }

    #[test]
    fn archive_has_scope_preview_undo_and_audit() {
        let mut agent = agent([Tool::Archive]);
        let pending = agent
            .propose_reply(r#"{"tool":"archive","threads":["t1","t2"]}"#)
            .unwrap();
        assert_eq!(pending.preview.text, "Archive 2 conversation(s)");
        assert!(!pending.preview.needs_confirmation);
        assert!(pending.undo.reversible);
        let applied = agent.commit(pending, Confirmation::NotAsked).unwrap();
        assert_eq!(
            applied,
            vec![Submission::Archive {
                threads: vec![ThreadId::new("t1"), ThreadId::new("t2")]
            }]
        );
        let undo = agent.undo().unwrap();
        assert_eq!(
            undo[1],
            Submission::MoveTo {
                thread: ThreadId::new("t2"),
                mailbox: MailboxId::new("work"),
            }
        );
        assert!(agent.undo().is_none());
        let lines: Vec<String> = agent.audit().iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "call=1 tool=archive threads=2 outcome=proposed",
                "call=1 tool=archive threads=2 outcome=committed",
                "call=1 tool=archive threads=2 outcome=undone",
            ]
        );
    }

    #[test]
    fn the_policy_and_the_scope_fail_closed() {
        let mut agent = agent([Tool::Summarize]);
        let denied = agent.propose(ToolCall::Archive {
            threads: vec![ThreadId::new("t1")],
        });
        assert!(matches!(denied, Err(Error::ToolDenied)));
        let outside = agent.propose(ToolCall::Summarize {
            thread: ThreadId::new("elsewhere"),
        });
        assert!(matches!(outside, Err(Error::OutOfScope)));
        let mut archive = Agent::new(Policy::new([Tool::Archive]), scope());
        assert!(matches!(
            archive.propose(ToolCall::Archive { threads: vec![] }),
            Err(Error::ToolDenied)
        ));
        assert!(
            agent
                .audit()
                .iter()
                .all(|line| line.outcome == Outcome::Denied)
        );
        assert_eq!(agent.audit().len(), 2);
    }

    #[test]
    fn send_and_delete_wait_for_the_app_confirmation() {
        let mut agent = agent([Tool::Send, Tool::Delete]);
        let reply = r#"{"tool":"send","to":[{"name":null,"email":"ana@acme.io"}],
            "subject":"Hi","body":"secret words"}"#;
        let pending = agent.propose_reply(reply).unwrap();
        assert!(pending.preview.needs_confirmation);
        assert_eq!(pending.preview.recipients, ["ana@acme.io"]);
        assert!(!pending.undo.reversible);
        assert!(matches!(
            agent.commit(pending.clone(), Confirmation::NotAsked),
            Err(Error::NeedsConfirmation)
        ));
        let sent = agent.commit(pending, Confirmation::Confirmed).unwrap();
        assert!(matches!(sent[0], Submission::Send { .. }));
        // A send leaves nothing to undo here.
        assert!(agent.undo().is_none());

        let delete = agent
            .propose(ToolCall::Delete {
                threads: vec![ThreadId::new("t1")],
            })
            .unwrap();
        assert!(matches!(
            agent.commit(delete, Confirmation::NotAsked),
            Err(Error::NeedsConfirmation)
        ));
        let audit = agent
            .audit()
            .iter()
            .map(ToString::to_string)
            .collect::<String>();
        assert!(!audit.contains("secret"));
        assert!(!audit.contains("ana@acme.io"));
        assert!(audit.contains("outcome=unconfirmed"));
    }

    #[test]
    fn a_caller_that_cannot_confirm_gets_send_held_for_the_app() {
        let mut agent = agent([Tool::Send, Tool::Summarize]);
        let read = agent
            .request(ToolCall::Summarize {
                thread: ThreadId::new("t1"),
            })
            .unwrap();
        assert!(matches!(read, Requested::Applied(ref s) if s.len() == 1));
        let held = agent
            .request(ToolCall::Send {
                to: vec![],
                subject: "Hi".into(),
                body: "Body".into(),
            })
            .unwrap();
        let Requested::Held { call, preview } = held else {
            panic!("send ran without the app");
        };
        assert!(preview.needs_confirmation);
        assert_eq!(agent.held().len(), 1);
        assert!(matches!(
            agent.approve(call, Confirmation::NotAsked),
            Err(Error::NeedsConfirmation)
        ));
        assert_eq!(agent.held().len(), 1);
        assert!(matches!(
            agent.approve(99, Confirmation::Confirmed),
            Err(Error::ToolDenied)
        ));
        let sent = agent.approve(call, Confirmation::Confirmed).unwrap();
        assert!(matches!(sent[0], Submission::Send { .. }));
        assert!(agent.held().is_empty());
    }

    #[test]
    fn free_text_forward_and_extra_fields_are_not_calls() {
        let mut agent = agent([Tool::Send, Tool::Forward, Tool::Archive]);
        for reply in [
            "tool:archive",
            "please archive t1",
            r#"{"tool":"forward","thread":"t1","to":[]}"#,
            r#"{"tool":"archive","threads":["t1"],"confirmed":true}"#,
            r#"{"tool":"archive","threads":["t1"]} {"tool":"archive","threads":["t2"]}"#,
        ] {
            assert!(
                matches!(parse_call(reply), Err(Error::ToolDenied)),
                "{reply}"
            );
            assert!(agent.propose_reply(reply).is_err());
        }
        assert_eq!(agent.audit().len(), 5);
        assert_eq!(
            agent.audit()[0].to_string(),
            "call=0 tool=unknown threads=0 outcome=denied"
        );
    }
}
