//! Pure domain: sync state machines, threading, the op queue and rules.
//!
//! I/O stays behind traits in `mailune-protocol`. This crate depends on the
//! contract and on nothing that opens a socket, a file or a process.

mod cancel;
mod followup;
mod lock;
mod notify;
mod queue;
mod schedule;
mod search;
mod thread;

pub use cancel::{CancelToken, Progress, run_loop};
pub use followup::{Exchange, awaiting_reply};
pub use lock::{AppLock, BiometricRequest};
pub use notify::{Decision, NoticePolicy, QuietHours};
pub use queue::{IdempotencyKey, Op, Pending, Queue, When};
pub use schedule::{Hold, Power, SyncAccount, SyncPlan, plan, plan_for};
pub use search::{Date, Query, SearchDoc, Term, fuse, parse_query};
pub use thread::{Container, NormalizedSubject, Threadable, normalize_subject, thread_messages};

/// Failure returned by the domain.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The contract crate refused the call.
    #[error(transparent)]
    Contract(#[from] mailune_protocol::Error),
    /// A [`CancelToken`] stopped the work at a checkpoint.
    #[error("cancelled")]
    Cancelled,
    /// A search token was not one this parser accepts.
    #[error("bad search token: {token}")]
    BadQuery {
        /// The token that failed, without the rest of the query.
        token: String,
    },
    /// The same idempotency key was used for a different operation.
    #[error("idempotency key reused for a different operation")]
    KeyMismatch,
    /// Undo was asked for after its window closed.
    #[error("undo window has closed")]
    UndoExpired,
    /// The queue has no operation with that identity.
    #[error("unknown operation")]
    UnknownOp,
    /// A move conflict was asked for a snooze, reminder, or reply-later.
    #[error("operation is not a move")]
    NotAMove,
    /// Send was asked for while the app is locked.
    #[error("app is locked")]
    Locked,
}

/// Confirms the domain links through to the contract.
pub fn ready() -> Result<(), Error> {
    mailune_protocol::ready()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ready;

    #[test]
    fn ready_reaches_the_contract() {
        assert!(ready().is_ok());
    }
}
