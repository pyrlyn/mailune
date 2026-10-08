//! `mailune-app` exported through UniFFI, for the Swift, Kotlin and C# shells
//! that link the core in-process.
//!
//! A surface only forwards: each export body is one expression that calls
//! assembly, and `crates/mailune-cli/tests/conventions.rs` parses this crate
//! to hold that. Records ([`records`]) are copies of the contract and view
//! types, so the contract carries no binding derives. The shell's keychain
//! reaches the core through the [`HostSecrets`] foreign trait ([`host`]).
//! Errors are one enum, [`MailuneError`].
//!
//! `unsafe_code` stays `deny`. The scaffolding the UniFFI macros expand is
//! unsafe by nature, but rustc does not apply the lint to expansions of
//! another crate's macros (as ketch-ffi records for uniffi 0.32.2), and code
//! written here is still linted.

uniffi::setup_scaffolding!();

pub mod error;
pub mod host;
pub mod records;

use std::sync::{Arc, Mutex, PoisonError};

use mailune_app::Views;
use mailune_protocol as proto;

pub use error::MailuneError;
pub use host::HostSecrets;
pub use records::{
    Address, Category, ComposerDraft, Event, SecretKind, Settings, ThreadRow, ViewState,
};

use host::ForeignSecrets;

/// Whether a bearer token is stored for `account`, asked through the shell's
/// keychain. The answer is a bool, so token bytes never reach the UI.
#[uniffi::export]
pub async fn has_token(
    secrets: Arc<dyn HostSecrets>,
    account: String,
) -> Result<bool, MailuneError> {
    Ok(mailune_app::has_token(&ForeignSecrets(secrets), &proto::AccountId::new(account)).await?)
}

/// The version of the core this library was built from.
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The view models one shell window renders. Safe to share between threads.
///
/// The lock is held only while events fold into memory, never across I/O or
/// a foreign call.
#[derive(Debug, Default, uniffi::Object)]
pub struct MailuneCore {
    views: Mutex<Views>,
}

impl MailuneCore {
    /// Runs `apply` on the views and returns what the shell should render.
    fn with_views(&self, apply: impl FnOnce(&mut Views)) -> ViewState {
        // A panic inside a fold leaves plain data behind, not a broken
        // invariant, so a poisoned lock is still safe to read.
        let mut views = self.views.lock().unwrap_or_else(PoisonError::into_inner);
        apply(&mut views);
        ViewState::from(&*views)
    }
}

#[uniffi::export]
impl MailuneCore {
    /// Empty list, no open thread, empty draft, English on the free plan.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Applies `events` in order and returns the state to render.
    pub fn fold(&self, events: Vec<Event>) -> ViewState {
        self.with_views(|views| views.fold(&events.into_iter().map(Into::into).collect::<Vec<_>>()))
    }

    /// The state to render now.
    pub fn state(&self) -> ViewState {
        self.with_views(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use super::{
        Address, Category, Event, HostSecrets, MailuneCore, MailuneError, SecretKind, ThreadRow,
        core_version, has_token,
    };

    fn drive<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the fake keychain never waits"),
        }
    }

    /// In-memory keychain. It never calls the real one.
    #[derive(Default)]
    struct Keychain {
        token: Mutex<Option<Vec<u8>>>,
        broken: bool,
    }

    impl HostSecrets for Keychain {
        fn get(
            &self,
            kind: SecretKind,
            account: Option<String>,
        ) -> Result<Option<Vec<u8>>, MailuneError> {
            if self.broken {
                return Err(MailuneError::Host {
                    message: "keychain locked".into(),
                });
            }
            assert_eq!(account.as_deref(), Some("ada"));
            Ok(match kind {
                SecretKind::Token => self.token.lock().unwrap().clone(),
                SecretKind::Password | SecretKind::DbKey => None,
            })
        }

        fn put(
            &self,
            _kind: SecretKind,
            _account: Option<String>,
            secret: Vec<u8>,
        ) -> Result<(), MailuneError> {
            *self.token.lock().unwrap() = Some(secret);
            Ok(())
        }

        fn delete(&self, _kind: SecretKind, _account: Option<String>) -> Result<(), MailuneError> {
            *self.token.lock().unwrap() = None;
            Ok(())
        }
    }

    fn row(id: &str) -> ThreadRow {
        ThreadRow {
            id: id.into(),
            account: "local".into(),
            from: Address {
                name: None,
                email: "ada@example.com".into(),
            },
            subject: "Hello".into(),
            snippet: "plain".into(),
            stamp: "t0".into(),
            message_count: 1,
            unread: true,
            flagged: false,
            important: false,
            pinned: false,
            snoozed: false,
            draft: false,
            has_attachment: false,
            category: Category::Primary,
            mailbox: "inbox".into(),
            labels: Vec::new(),
        }
    }

    #[test]
    fn the_async_export_answers_through_the_foreign_keychain() {
        let keychain = Arc::new(Keychain::default());
        assert!(!drive(has_token(keychain.clone(), "ada".into())).unwrap());
        keychain
            .put(SecretKind::Token, None, b"t".to_vec())
            .unwrap();
        assert!(drive(has_token(keychain, "ada".into())).unwrap());
    }

    #[test]
    fn a_keychain_failure_is_a_host_error_with_the_operation() {
        let keychain = Arc::new(Keychain {
            broken: true,
            ..Keychain::default()
        });
        let err = drive(has_token(keychain, "ada".into())).unwrap_err();
        assert_eq!(
            err,
            MailuneError::Host {
                message: "secrets.get: keychain locked".into()
            }
        );
    }

    #[test]
    fn folding_events_returns_the_state_to_render() {
        let core = MailuneCore::new();
        assert!(core.state().list.is_empty());
        let state = core.fold(vec![
            Event::Snapshot {
                threads: vec![row("t1"), row("t2")],
            },
            Event::Notice {
                message: "open t2".into(),
            },
            Event::Notice {
                message: "settings\nlanguage: fr\nplan: plus".into(),
            },
        ]);
        assert_eq!(state.list.len(), 2);
        assert_eq!(state.open.map(|open| open.id), Some("t2".to_string()));
        assert_eq!(state.settings.language, "fr");
        assert_eq!(core.state().settings.plan, "plus");
    }

    #[test]
    fn the_version_is_the_crate_version() {
        assert_eq!(core_version(), env!("CARGO_PKG_VERSION"));
    }
}
