//! Pure domain: sync state machines, threading, the op queue and rules.
//!
//! I/O stays behind traits in `mailune-protocol`. This crate depends on the
//! contract and on nothing that opens a socket, a file or a process.

/// Failure returned by the domain.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The contract crate refused the call.
    #[error(transparent)]
    Contract(#[from] mailune_protocol::Error),
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
