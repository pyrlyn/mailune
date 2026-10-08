//! Command-line surface. Dispatches into `mailune-app` and nothing else.
//!
//! Library crates keep their own error types. Only this binary uses `anyhow`.

fn main() -> anyhow::Result<()> {
    mailune_app::ready()?;
    Ok(())
}
