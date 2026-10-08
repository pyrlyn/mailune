//! The one error enum every export returns and every foreign trait may raise.
//!
//! The text of each variant is safe to show and log: assembly already keeps
//! token bytes out of its errors, and a host is required to do the same for
//! the messages it raises.

/// Failure crossing the binding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MailuneError {
    /// The host (keychain, network, platform) failed. Raised by a foreign
    /// implementation too, so its text must not carry a secret.
    #[error("{message}")]
    Host {
        /// What went wrong, with no secret material.
        message: String,
    },
    /// No bearer token is stored for this account.
    #[error("no token is stored for {account}")]
    MissingToken {
        /// The account id. A name, not the token.
        account: String,
    },
    /// The domain refused the call.
    #[error("{message}")]
    Core {
        /// Why the domain refused.
        message: String,
    },
}

impl From<mailune_app::Error> for MailuneError {
    fn from(err: mailune_app::Error) -> Self {
        match err {
            mailune_app::Error::MissingToken { account } => Self::MissingToken { account },
            mailune_app::Error::Secret(err) => Self::Host {
                message: err.to_string(),
            },
            mailune_app::Error::Core(err) => Self::Core {
                message: err.to_string(),
            },
        }
    }
}

/// A foreign implementation that threw something other than [`MailuneError`]
/// still has to become an error, not a panic across the boundary.
impl From<uniffi::UnexpectedUniFFICallbackError> for MailuneError {
    fn from(err: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Host {
            message: err.reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MailuneError;

    #[test]
    fn a_missing_token_keeps_the_account_name() {
        let err = MailuneError::from(mailune_app::Error::MissingToken {
            account: "ada".into(),
        });
        assert_eq!(
            err,
            MailuneError::MissingToken {
                account: "ada".into()
            }
        );
    }

    #[test]
    fn a_host_failure_keeps_its_safe_text() {
        let err = MailuneError::from(mailune_app::Error::Secret(mailune_protocol::Error::host(
            "secrets.get",
            "locked",
        )));
        assert_eq!(err.to_string(), "secrets.get: locked");
    }

    #[test]
    fn an_unexpected_callback_error_becomes_a_host_error() {
        let err = MailuneError::from(uniffi::UnexpectedUniFFICallbackError::new("boom"));
        assert_eq!(
            err,
            MailuneError::Host {
                message: "boom".into()
            }
        );
    }
}
