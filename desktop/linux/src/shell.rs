//! Three-pane shell description.
//!
//! The names and the breakpoint live in `share/shell.desc`. This file only
//! reads them. GTK stays off.

use crate::Error;

/// The panes a wide window shows, and the width where they collapse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    /// Pane names, in order.
    pub panes: Vec<String>,
    /// Width, in pixels, below which the panes collapse.
    pub breakpoint: u32,
}

/// Reads pane names and one breakpoint from `text`.
///
/// # Errors
///
/// [`Error::Description`] when there are not three panes or no breakpoint.
pub fn load_shell(text: &str) -> Result<Shell, Error> {
    let mut panes = Vec::new();
    let mut breakpoint = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        if key == "pane" {
            panes.push(value.to_string());
        } else if key == "breakpoint" {
            breakpoint = value.parse().ok();
        }
    }
    let Some(breakpoint) = breakpoint else {
        return Err(Error::Description);
    };
    if panes.len() != 3 {
        return Err(Error::Description);
    }
    Ok(Shell { panes, breakpoint })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_shell;

    #[test]
    fn the_description_names_three_panes_and_a_breakpoint() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/shell.desc");
        let text = std::fs::read_to_string(path).unwrap();
        let shell = load_shell(&text).unwrap();
        assert_eq!(shell.panes, ["folders", "messages", "reader"]);
        assert_eq!(shell.breakpoint, 900);
    }
}
