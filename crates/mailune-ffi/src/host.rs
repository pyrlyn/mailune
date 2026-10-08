//! What the shell implements: the keychain, as a foreign trait that becomes
//! the contract's `SecretStore`.
//!
//! A foreign trait (`export(foreign)`) rather than a callback interface:
//! UniFFI calls the latter soft-deprecated, and a foreign trait crosses as an
//! `Arc` the shell can hand to several calls. The methods are synchronous on
//! the foreign side because every shell keychain API is; the adapter wraps
//! them in the contract's async signature.

use std::sync::Arc;

use mailune_protocol as proto;

use crate::error::MailuneError;
use crate::records::SecretKind;

/// The OS keychain, implemented by the shell. Bytes cross only between this
/// trait and the core; no export hands them back to the UI.
#[uniffi::export(foreign)]
pub trait HostSecrets: Send + Sync {
    /// The bytes stored for `kind` and `account`, or `None` when absent.
    /// `account` is `None` only for the database key.
    fn get(
        &self,
        kind: SecretKind,
        account: Option<String>,
    ) -> Result<Option<Vec<u8>>, MailuneError>;

    /// Stores `secret`, replacing a previous value.
    fn put(
        &self,
        kind: SecretKind,
        account: Option<String>,
        secret: Vec<u8>,
    ) -> Result<(), MailuneError>;

    /// Forgets the secret. An absent one is success.
    fn delete(&self, kind: SecretKind, account: Option<String>) -> Result<(), MailuneError>;
}

/// The shell's keychain, as the contract's store.
pub(crate) struct ForeignSecrets(pub(crate) Arc<dyn HostSecrets>);

fn parts(id: &proto::SecretId) -> (SecretKind, Option<String>) {
    (
        id.kind().into(),
        id.account().map(|account| account.as_str().to_string()),
    )
}

/// The foreign text is passed through: the trait's contract keeps secrets
/// out of it, and dropping it would hide why the keychain failed.
fn host_error(operation: &'static str, err: MailuneError) -> proto::Error {
    proto::Error::host(operation, err.to_string())
}

impl proto::SecretStore for ForeignSecrets {
    async fn get(&self, id: &proto::SecretId) -> Result<Option<proto::Secret>, proto::Error> {
        let (kind, account) = parts(id);
        self.0
            .get(kind, account)
            .map(|bytes| bytes.map(proto::Secret::new))
            .map_err(|err| host_error("secrets.get", err))
    }

    async fn put(&self, id: &proto::SecretId, secret: &proto::Secret) -> Result<(), proto::Error> {
        let (kind, account) = parts(id);
        self.0
            .put(kind, account, secret.as_bytes().to_vec())
            .map_err(|err| host_error("secrets.put", err))
    }

    async fn delete(&self, id: &proto::SecretId) -> Result<(), proto::Error> {
        let (kind, account) = parts(id);
        self.0
            .delete(kind, account)
            .map_err(|err| host_error("secrets.delete", err))
    }
}
