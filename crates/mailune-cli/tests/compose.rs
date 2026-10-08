//! Reads the integration compose file.
//!
//! The test checks service names in the YAML text. It does not start Docker
//! or open a socket.

const COMPOSE: &str = include_str!("../../../compose.yaml");

#[test]
fn stalwart_and_dovecot_are_the_services() {
    assert_eq!(service_names(COMPOSE), ["stalwart", "dovecot"]);
}

fn service_names(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut in_services = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            in_services = trimmed == "services:";
            continue;
        }
        if !in_services {
            continue;
        }
        let Some(name) = line.strip_prefix("  ") else {
            continue;
        };
        if name.starts_with(' ') || name.starts_with('\t') {
            continue;
        }
        let Some(name) = name.strip_suffix(':') else {
            continue;
        };
        if !name.is_empty() && !name.contains(' ') {
            names.push(name);
        }
    }
    names
}
