//! Assembly: owns config and the runtime, and folds events into view models.
//!
//! No CLI and no printing. Surfaces call this crate; it calls the domain.
//! Token bytes are read through [`SecretStore`] and never placed on a `Debug`
//! or log path.

mod gettext;
mod secrets;
mod views;

use mailune_protocol::{AccountId, Secret, SecretId, SecretStore};

pub use gettext::gettext;
pub use secrets::{
    AccountConfig, AndroidSecretCall, FakeSecretStore, read_db_key, read_password, store_db_key,
    store_password,
};
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
    /// No password is stored for this account.
    ///
    /// The account id is a name, not the password.
    #[error("no password is stored for {account}")]
    MissingPassword {
        /// Account that has no password.
        account: String,
    },
    /// No database key is stored.
    #[error("no database key is stored")]
    MissingDbKey,
    /// An Android put callback arrived without a secret.
    #[error("android put needs a secret")]
    AndroidPutMissing,
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

/// Log text for a token that was just read.
///
/// Uses [`Secret`]'s `Debug`, which does not include [`Secret::as_bytes`].
pub fn token_log_line(account: &AccountId, secret: &Secret) -> String {
    format!("token for {} is {secret:?}", account.as_str())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use mailune_protocol::{AccountId, Secret, SecretId, SecretKind, SecretStore};

    use super::{read_token, token_log_line};

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
        let line = token_log_line(&account, &secret);
        assert!(!line.contains(text), "{line}");
        assert!(!format!("{secret:?}").contains(text));
        assert!(line.contains("redacted"), "{line}");
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
