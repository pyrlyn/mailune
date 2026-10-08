//! Relay client.
//!
//! A device registers for an account. A wake marks that account due for sync.
//! Nothing here opens a socket, and the client keeps no token or mail text.

use crate::{Error, Wake};

/// One device registered for one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// Device id from the caller. Not a push token.
    pub device_id: String,
    /// Account the device syncs.
    pub account_id: String,
}

/// Registrations and the accounts a wake has marked due.
#[derive(Debug, Default)]
pub struct Client {
    devices: Vec<Registration>,
    due: Vec<String>,
}

impl Client {
    /// No devices and nothing due.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `device_id` for `account_id`, replacing a previous row for that device.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] or [`Error::Account`] when an id is empty.
    pub fn register(&mut self, device_id: &str, account_id: &str) -> Result<(), Error> {
        if device_id.is_empty() || device_id.chars().any(char::is_whitespace) {
            return Err(Error::Device);
        }
        if account_id.is_empty() || account_id.chars().any(char::is_whitespace) {
            return Err(Error::Account);
        }
        self.devices.retain(|device| device.device_id != device_id);
        self.devices.push(Registration {
            device_id: device_id.to_string(),
            account_id: account_id.to_string(),
        });
        Ok(())
    }

    /// Marks `wake`'s account due when a device is registered for it.
    pub fn apply(&mut self, wake: &Wake) {
        let registered = self
            .devices
            .iter()
            .any(|device| device.account_id == wake.account_id);
        if registered && !self.due.iter().any(|id| id == &wake.account_id) {
            self.due.push(wake.account_id.clone());
        }
    }

    /// Accounts waiting for sync, in the order they became due.
    pub fn due(&self) -> &[String] {
        &self.due
    }

    /// Drops `account_id` from the due list after a sync.
    pub fn ack(&mut self, account_id: &str) {
        self.due.retain(|id| id != account_id);
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use crate::Wake;

    #[test]
    fn a_registered_account_becomes_due_and_an_unknown_one_does_not() {
        let mut client = Client::new();
        client.register("device-1", "acc-1").unwrap();
        client.apply(&Wake {
            account_id: "acc-1".into(),
        });
        client.apply(&Wake {
            account_id: "acc-1".into(),
        });
        client.apply(&Wake {
            account_id: "other".into(),
        });
        assert_eq!(client.due(), ["acc-1"]);
        client.ack("acc-1");
        assert!(client.due().is_empty());
    }

    #[test]
    fn an_empty_device_id_is_rejected() {
        let mut client = Client::new();
        let err = client.register("", "acc-1").unwrap_err();
        assert!(matches!(err, crate::Error::Device));
    }
}
