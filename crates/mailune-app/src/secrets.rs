//! Passwords and the database key.
//!
//! Both go through [`SecretStore`] get and put, the same path as the bearer
//! token. [`FakeSecretStore::android`] is the host callback a mobile shell
//! would call. This module does not open an OS secret store. A shared secret
//! package is not in this repo.

use std::collections::HashMap;
use std::sync::Mutex;

use mailune_protocol::{AccountId, Secret, SecretId, SecretStore};

use crate::Error;

/// What the mobile host callback is asking the store to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidSecretCall {
    /// Read the secret named by the id.
    Get,
    /// Replace the secret named by the id.
    Put,
}

/// Account settings that are safe to write next to the app config.
///
/// A password and the database key are not fields here. They stay in the secret store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountConfig {
    /// Account name. Not a credential.
    pub account: String,
    /// Mail host. Not a credential.
    pub host: String,
}

impl AccountConfig {
    /// Builds config from the account name and host only.
    #[must_use]
    pub fn new(account: &AccountId, host: impl Into<String>) -> Self {
        Self {
            account: account.as_str().to_string(),
            host: host.into(),
        }
    }
}

/// In-memory secret store for tests and for a shell that has no OS store yet.
///
/// `put` keeps the bytes. Nothing is written to a config file.
#[derive(Debug, Default)]
pub struct FakeSecretStore {
    secrets: Mutex<HashMap<SecretId, Secret>>,
}

impl FakeSecretStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Android host callback. `secret` is required for [`AndroidSecretCall::Put`].
    ///
    /// # Errors
    ///
    /// [`Error::AndroidPutMissing`] when a put has no secret.
    pub fn android(
        &self,
        call: AndroidSecretCall,
        id: &SecretId,
        secret: Option<&Secret>,
    ) -> Result<Option<Secret>, Error> {
        let mut guard = self.lock();
        match call {
            AndroidSecretCall::Get => Ok(guard.get(id).cloned()),
            AndroidSecretCall::Put => {
                let Some(secret) = secret else {
                    return Err(Error::AndroidPutMissing);
                };
                guard.insert(id.clone(), secret.clone());
                Ok(None)
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<SecretId, Secret>> {
        self.secrets.lock().unwrap_or_else(|err| err.into_inner())
    }
}

impl SecretStore for FakeSecretStore {
    async fn get(&self, id: &SecretId) -> Result<Option<Secret>, mailune_protocol::Error> {
        Ok(self.lock().get(id).cloned())
    }

    async fn put(&self, id: &SecretId, secret: &Secret) -> Result<(), mailune_protocol::Error> {
        self.lock().insert(id.clone(), secret.clone());
        Ok(())
    }

    async fn delete(&self, id: &SecretId) -> Result<(), mailune_protocol::Error> {
        self.lock().remove(id);
        Ok(())
    }
}

/// Stores the password for `account`.
///
/// # Errors
///
/// [`Error::Secret`] when the store cannot write.
pub async fn store_password(
    store: &impl SecretStore,
    account: &AccountId,
    password: &Secret,
) -> Result<(), Error> {
    store
        .put(&SecretId::password(account.clone()), password)
        .await?;
    Ok(())
}

/// Reads the password for `account`.
///
/// # Errors
///
/// [`Error::MissingPassword`] when nothing is stored, and [`Error::Secret`]
/// when the store itself fails. The error names the account, not the password.
pub async fn read_password(store: &impl SecretStore, account: &AccountId) -> Result<Secret, Error> {
    match store.get(&SecretId::password(account.clone())).await? {
        Some(secret) => Ok(secret),
        None => Err(Error::MissingPassword {
            account: account.as_str().to_string(),
        }),
    }
}

/// Stores the single database key.
///
/// # Errors
///
/// [`Error::Secret`] when the store cannot write.
pub async fn store_db_key(store: &impl SecretStore, key: &Secret) -> Result<(), Error> {
    store.put(&SecretId::db_key(), key).await?;
    Ok(())
}

/// Reads the database key.
///
/// # Errors
///
/// [`Error::MissingDbKey`] when nothing is stored, and [`Error::Secret`] when
/// the store itself fails.
pub async fn read_db_key(store: &impl SecretStore) -> Result<Secret, Error> {
    match store.get(&SecretId::db_key()).await? {
        Some(secret) => Ok(secret),
        None => Err(Error::MissingDbKey),
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use mailune_protocol::{AccountId, Secret, SecretId};

    use super::{
        AccountConfig, AndroidSecretCall, FakeSecretStore, read_db_key, read_password,
        store_db_key, store_password,
    };

    fn drive<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("secret future waited"),
        }
    }

    #[test]
    fn a_password_and_db_key_round_trip_and_stay_out_of_config() {
        let store = FakeSecretStore::new();
        let account = AccountId::new("ada");
        let password = Secret::new(b"pw-NOT-IN-CONFIG-9c2e");
        let key = Secret::new(b"db-NOT-IN-CONFIG-4b11");
        drive(store_password(&store, &account, &password)).unwrap();
        drive(store_db_key(&store, &key)).unwrap();

        let read = drive(read_password(&store, &account)).unwrap();
        assert_eq!(read.as_bytes(), password.as_bytes());
        let read_key = drive(read_db_key(&store)).unwrap();
        assert_eq!(read_key.as_bytes(), key.as_bytes());

        let secret_debug = format!("{read:?}");
        assert!(secret_debug.contains("redacted"), "{secret_debug}");
        assert!(!secret_debug.contains("NOT-IN-CONFIG"));

        let config = AccountConfig::new(&account, "imap.example");
        let config_debug = format!("{config:?}");
        assert_eq!(config.account, "ada");
        assert_eq!(config.host, "imap.example");
        assert!(!config_debug.contains("NOT-IN-CONFIG"), "{config_debug}");
        assert!(!config_debug.contains("pw-"));
        assert!(!config_debug.contains("db-"));
    }

    #[test]
    fn the_android_callback_stores_through_the_fake() {
        let store = FakeSecretStore::new();
        let account = AccountId::new("ada");
        let id = SecretId::password(account);
        let password = Secret::new(b"android-secret-value");
        let missing = store
            .android(AndroidSecretCall::Put, &id, None)
            .unwrap_err();
        assert!(!missing.to_string().contains("android-secret"));
        store
            .android(AndroidSecretCall::Put, &id, Some(&password))
            .unwrap();
        let got = store.android(AndroidSecretCall::Get, &id, None).unwrap();
        let got = got.expect("stored");
        assert_eq!(got.as_bytes(), password.as_bytes());
        assert!(!format!("{got:?}").contains("android-secret"));
    }
}
