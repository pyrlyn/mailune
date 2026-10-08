//! Assembly: owns config and the runtime, and folds events into view models.
//!
//! No CLI and no printing. Surfaces call this crate; it calls the domain.
//! Adapters will be wired here once they exist.

/// Failure returned by assembly.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The domain refused the call.
    #[error(transparent)]
    Core(#[from] mailune_core::Error),
}

/// Confirms assembly links through to the domain.
pub fn ready() -> Result<(), Error> {
    mailune_core::ready()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ready;

    #[test]
    fn ready_reaches_the_domain() {
        assert!(ready().is_ok());
    }
}
