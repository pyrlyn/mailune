//! Reads `docker-compose.yml` at the repository root, the integration mail
//! servers. Nothing here starts Docker or opens a socket: the file is text.
//!
//! Helpers sit outside `#[test]`, so clippy does not treat them as test code
//! even with `allow-unwrap-in-tests`.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Each service with its `image`, `ports` and environment lines. Indentation
/// is the compose file's own two spaces per level; no YAML parser is needed
/// for a file this flat.
#[derive(Default, Debug)]
struct Service {
    image: Option<String>,
    ports: Vec<String>,
    environment: Vec<String>,
}

fn services() -> BTreeMap<String, Service> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docker-compose.yml");
    let text = std::fs::read_to_string(path).unwrap();
    let mut services: BTreeMap<String, Service> = BTreeMap::new();
    let (mut current, mut section) = (None::<String>, "");
    for line in text.lines() {
        let content = line.split(" #").next().unwrap_or("").trim_end();
        let trimmed = content.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = content.len() - trimmed.len();
        match indent {
            0 => current = None,
            2 => {
                let name = trimmed.trim_end_matches(':').to_string();
                services.entry(name.clone()).or_default();
                current = Some(name);
            }
            4 => {
                let Some(service) = current.as_ref().and_then(|name| services.get_mut(name)) else {
                    continue;
                };
                if let Some(image) = trimmed.strip_prefix("image:") {
                    service.image = Some(image.trim().to_string());
                }
                section = if trimmed.starts_with("ports:") {
                    "ports"
                } else if trimmed.starts_with("environment:") {
                    "environment"
                } else {
                    ""
                };
            }
            _ => {
                let Some(service) = current.as_ref().and_then(|name| services.get_mut(name)) else {
                    continue;
                };
                match section {
                    "ports" => service.ports.push(
                        trimmed
                            .trim_start_matches("- ")
                            .trim_matches('"')
                            .to_string(),
                    ),
                    "environment" => service.environment.push(trimmed.to_string()),
                    _ => {}
                }
            }
        }
    }
    services
}

#[test]
fn compose_names_stalwart_and_dovecot_on_pinned_images() {
    let services = services();
    let names: Vec<&str> = services.keys().map(String::as_str).collect();
    assert_eq!(names, ["dovecot", "stalwart"]);
    for (name, service) in &services {
        let image = service.image.as_deref().unwrap_or_default();
        let tag = image
            .rsplit_once(':')
            .map(|(_, tag)| tag)
            .unwrap_or_default();
        assert!(
            !tag.is_empty() && !tag.starts_with("latest"),
            "{name} must pin an image version, found {image:?}"
        );
        assert!(!service.ports.is_empty(), "{name} exposes no port");
    }
}

#[test]
fn every_port_binds_to_loopback_and_no_password_is_committed() {
    for (name, service) in services() {
        for port in &service.ports {
            assert!(
                port.starts_with("127.0.0.1:"),
                "{name} publishes {port} beyond loopback"
            );
        }
        for entry in &service.environment {
            if entry.contains("PASSWORD") {
                let value = entry.split_once(':').map(|(_, value)| value.trim());
                assert!(
                    value.is_some_and(|value| value.starts_with("${")),
                    "{name} commits a password: {entry}"
                );
            }
        }
    }
}
