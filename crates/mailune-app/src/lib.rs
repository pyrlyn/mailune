//! Assembly: owns config and the runtime, and folds events into view models.
//!
//! No CLI and no printing. Surfaces call this crate; it calls the domain.
//! Token bytes are read through [`SecretStore`] and never placed on a `Debug`
//! or log path.

mod calendar;
mod reducer;
mod views;

use mailune_protocol::{AccountId, Secret, SecretId, SecretStore};

pub use calendar::{CalendarDay, CalendarEntry, CalendarView, calendar_view};
pub use reducer::{Msg, Pending, Ui};
pub use views::{ComposerDraft, OpenThread, SettingsSnapshot, ThreadList, Views};

/// Failure returned by assembly.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The domain refused the call.
    #[error(transparent)]
    Core(#[from] mailune_core::Error),
    /// The host secret store failed.
    ///
    /// The protocol error's text is required to omit token bytes.
    #[error(transparent)]
    Secret(#[from] mailune_protocol::Error),
    /// No bearer token is stored for this account.
    ///
    /// The account id is a name, not the token.
    #[error("no token is stored for {account}")]
    MissingToken {
        /// Account that has no token.
        account: String,
    },
}

/// Confirms assembly links through to the domain.
///
/// # Errors
///
/// Returns [`Error::Core`] when the domain cannot be reached.
pub fn ready() -> Result<(), Error> {
    mailune_core::ready()?;
    Ok(())
}

/// Reads the bearer token for `account`.
///
/// The value is a [`Secret`]: its `Debug` is `Secret(redacted)` and it has no
/// `Display`. [`token_log_line`] is the log text for a successful read.
///
/// # Errors
///
/// Returns [`Error::MissingToken`] when the store has no token, and
/// [`Error::Secret`] when the store itself fails.
pub async fn read_token(store: &impl SecretStore, account: &AccountId) -> Result<Secret, Error> {
    match store.get(&SecretId::token(account.clone())).await? {
        Some(secret) => Ok(secret),
        None => Err(Error::MissingToken {
            account: account.as_str().to_string(),
        }),
    }
}

/// Whether a bearer token is stored for `account`.
///
/// Front ends ask this to choose between the inbox and sign-in. The answer is
/// a bool so the token bytes never cross into a surface.
///
/// # Errors
///
/// Returns [`Error::Secret`] when the store itself fails.
pub async fn has_token(store: &impl SecretStore, account: &AccountId) -> Result<bool, Error> {
    match read_token(store, account).await {
        Ok(_) => Ok(true),
        Err(Error::MissingToken { .. }) => Ok(false),
        Err(err) => Err(err),
    }
}

/// Log text for a token that was just read.
///
/// The account id is a name. The secret is not a parameter, so the line cannot
/// contain token bytes.
pub fn token_log_line(account: &AccountId) -> String {
    format!("token read for {}", account.as_str())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use mailune_protocol::{AccountId, Secret, SecretId, SecretKind, SecretStore};

    use super::{has_token, read_token, token_log_line};

    fn drive<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("fake secret future waited"),
        }
    }

    /// In-memory store. It never calls the keychain.
    struct Mem {
        token: Option<Secret>,
    }

    impl SecretStore for Mem {
        async fn get(&self, id: &SecretId) -> Result<Option<Secret>, mailune_protocol::Error> {
            if id.kind() == SecretKind::Token {
                Ok(self.token.clone())
            } else {
                Ok(None)
            }
        }

        async fn put(
            &self,
            _id: &SecretId,
            _secret: &Secret,
        ) -> Result<(), mailune_protocol::Error> {
            Ok(())
        }

        async fn delete(&self, _id: &SecretId) -> Result<(), mailune_protocol::Error> {
            Ok(())
        }
    }

    #[test]
    fn ready_reaches_the_domain() {
        assert!(super::ready().is_ok());
    }

    #[test]
    fn log_line_does_not_include_the_token_bytes() {
        let raw = b"token-bytes-NOT-FOR-LOGS-7f3a";
        let store = Mem {
            token: Some(Secret::new(raw)),
        };
        let account = AccountId::new("ada");
        let secret = drive(read_token(&store, &account)).unwrap();
        assert_eq!(secret.as_bytes(), raw);
        let text = std::str::from_utf8(raw).unwrap();
        let line = token_log_line(&account);
        assert!(!line.contains(text));
        assert!(line.contains(account.as_str()));
    }

    #[test]
    fn has_token_answers_without_the_bytes() {
        let account = AccountId::new("ada");
        let stored = Mem {
            token: Some(Secret::new(b"t")),
        };
        assert!(drive(has_token(&stored, &account)).unwrap());
        assert!(!drive(has_token(&Mem { token: None }, &account)).unwrap());
    }

    #[test]
    fn missing_token_names_the_account_only() {
        let store = Mem { token: None };
        let account = AccountId::new("ada");
        let err = drive(read_token(&store, &account)).unwrap_err();
        let rendered = err.to_string();
        assert!(rendered.contains("ada"), "{rendered}");
        assert!(!rendered.contains("NOT-FOR-LOGS"));
    }
}
