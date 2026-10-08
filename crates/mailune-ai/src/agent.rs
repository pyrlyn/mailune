//! Agent tools with a scope, a preview, an undo record, and an audit line.
//!
//! [`Policy`](crate::Policy) still decides, and an empty allow-list denies
//! the call. Send, delete, and forward also need the app confirmation flag.
//! The undo record stays here: this module does not touch the core queue.

use crate::guard::tool_name;
use crate::{Error, Policy, Tool, ToolProposal};

/// Where a tool may act. A folder the person did not name is not implied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Mailbox or label the call is limited to.
    pub folder: String,
}

/// One proposed tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCall {
    /// The tool the model named.
    pub tool: Tool,
    /// The folder the call may touch.
    pub scope: Scope,
    /// The app confirmation flag. Send, delete, and forward require it.
    pub confirmed: bool,
}

/// What the call would do, before it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPreview {
    /// A short description. It does not include mail text.
    pub summary: String,
}

/// Enough to reverse a call later. Applying it is the app's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoRecord {
    /// The tool that was prepared.
    pub tool: Tool,
    /// The folder the call was scoped to.
    pub folder: String,
}

/// One line in the audit log. The outcome is a fixed word, not mail text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLine {
    /// Tool name.
    pub tool: &'static str,
    /// Folder from the scope.
    pub folder: String,
    /// `allowed` or `denied`.
    pub outcome: &'static str,
}

/// Audit lines in call order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AuditLog {
    lines: Vec<AuditLine>,
}

impl AuditLog {
    /// An empty log.
    #[must_use]
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Lines recorded so far.
    #[must_use]
    pub fn lines(&self) -> &[AuditLine] {
        &self.lines
    }
}

/// Checks the policy and the confirmation flag, then returns a preview and an undo record.
///
/// A denied call is still an audit line. The preview is not returned in that case.
///
/// # Errors
///
/// [`Error::ToolDenied`] when the policy does not list the tool, or when send,
/// delete, or forward is missing the confirmation flag.
pub fn prepare(
    policy: &Policy,
    call: &AgentCall,
    log: &mut AuditLog,
) -> Result<(ToolPreview, UndoRecord), Error> {
    let proposal = ToolProposal { tool: call.tool };
    if policy.approve(&proposal).is_err() {
        log.record(call, "denied");
        return Err(Error::ToolDenied);
    }
    if needs_confirmation(call.tool) && !call.confirmed {
        log.record(call, "denied");
        return Err(Error::ToolDenied);
    }
    log.record(call, "allowed");
    Ok((
        ToolPreview {
            summary: preview_text(call),
        },
        UndoRecord {
            tool: call.tool,
            folder: call.scope.folder.clone(),
        },
    ))
}

fn needs_confirmation(tool: Tool) -> bool {
    matches!(tool, Tool::Send | Tool::Delete | Tool::Forward)
}

fn preview_text(call: &AgentCall) -> String {
    let verb = match call.tool {
        Tool::Summarize => "read",
        Tool::Archive => "archive",
        Tool::Send => "send",
        Tool::Delete => "delete",
        Tool::Forward => "forward",
    };
    format!("{verb} mail in {}", call.scope.folder)
}

impl AuditLog {
    fn record(&mut self, call: &AgentCall, outcome: &'static str) {
        self.lines.push(AuditLine {
            tool: tool_name(call.tool),
            folder: call.scope.folder.clone(),
            outcome,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentCall, AuditLog, Scope, prepare};
    use crate::{Error, Policy, Tool};

    fn call(tool: Tool, confirmed: bool) -> AgentCall {
        AgentCall {
            tool,
            scope: Scope {
                folder: "Inbox".into(),
            },
            confirmed,
        }
    }

    #[test]
    fn the_policy_fails_closed_and_send_needs_confirmation() {
        let mut log = AuditLog::new();
        let denied = prepare(&Policy::new([]), &call(Tool::Summarize, true), &mut log);
        assert!(matches!(denied, Err(Error::ToolDenied)));
        assert_eq!(log.lines()[0].outcome, "denied");

        let policy = Policy::new([Tool::Summarize, Tool::Send]);
        let (preview, undo) = prepare(&policy, &call(Tool::Summarize, false), &mut log).unwrap();
        assert_eq!(preview.summary, "read mail in Inbox");
        assert_eq!(undo.tool, Tool::Summarize);
        assert_eq!(undo.folder, "Inbox");
        assert_eq!(log.lines()[1].outcome, "allowed");

        let unconfirmed = prepare(&policy, &call(Tool::Send, false), &mut log);
        assert!(matches!(unconfirmed, Err(Error::ToolDenied)));
        assert_eq!(log.lines()[2].tool, "send");
        assert_eq!(log.lines()[2].outcome, "denied");

        let (preview, undo) = prepare(&policy, &call(Tool::Send, true), &mut log).unwrap();
        assert_eq!(preview.summary, "send mail in Inbox");
        assert_eq!(undo.tool, Tool::Send);
        assert_eq!(log.lines()[3].outcome, "allowed");
        assert!(!preview.summary.contains("secret"));
    }
}
