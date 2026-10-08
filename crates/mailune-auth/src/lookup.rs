//! Autoconfig lookup over bytes and records the caller already has.
//!
//! [`ConfigSource`] returns ISPDB XML and the SRV and MX answers. This module
//! does not resolve a name. Host, port, and security for SRV and MX come from
//! the parsers in `autoconfig`: SRV security follows the service name, and an
//! MX answer has neither port nor security, so the guess is submission on 587
//! with STARTTLS and IMAP on 993 with implicit TLS.

use crate::autoconfig::{
    Autoconfig, MxRecord, ServerEndpoint, SrvRecord, SrvService, endpoint_from_mx,
    endpoint_from_srv, parse_mozilla_xml,
};
use crate::{Error, SocketSecurity};

/// Bytes and DNS answers the caller already fetched. No method here dials.
pub trait ConfigSource {
    /// Mozilla ISPDB XML for `domain`, or `None` when that document is absent.
    ///
    /// # Errors
    ///
    /// [`Error::Lookup`] when the source cannot answer. A missing document is
    /// `Ok(None)`, not an error, so SRV can run.
    fn ispdb_xml(&self, domain: &str) -> Result<Option<Vec<u8>>, Error>;

    /// SRV answers for one RFC 6186 service. Empty means the name had none.
    ///
    /// # Errors
    ///
    /// [`Error::Lookup`] when the source cannot answer.
    fn srv(&self, service: SrvService, domain: &str) -> Result<Vec<SrvRecord>, Error>;

    /// MX answers. Empty means the name had none.
    ///
    /// # Errors
    ///
    /// [`Error::Lookup`] when the source cannot answer.
    fn mx(&self, domain: &str) -> Result<Vec<MxRecord>, Error>;
}

/// Resolves servers for `email`.
///
/// ISPDB XML wins when the source has it. Otherwise the lowest-priority SRV
/// record for each protocol is used (`imaps` before `imap`, `submissions`
/// before `submission`). MX fills whatever side is still empty: the exchange
/// with the lowest preference, submission port 587 and STARTTLS for SMTP, and
/// the same host on 993 with implicit TLS for IMAP. Weight is not a dice roll;
/// within one priority the highest weight wins so a test is stable.
///
/// # Errors
///
/// [`Error::Lookup`] when `email` has no domain or nothing answered.
/// [`Error::Autoconfig`] when a document or record is not usable.
pub fn discover(source: &impl ConfigSource, email: &str) -> Result<Autoconfig, Error> {
    let domain = domain_of(email)?;
    if let Some(bytes) = source.ispdb_xml(domain)? {
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| Error::Lookup(format!("ISPDB for {domain} is not UTF-8")))?;
        return parse_mozilla_xml(text);
    }

    let mut incoming = first_srv(source, domain, &[SrvService::Imaps, SrvService::Imap])?;
    let mut outgoing = first_srv(
        source,
        domain,
        &[SrvService::Submissions, SrvService::Submission],
    )?;
    if (incoming.is_empty() || outgoing.is_empty())
        && let Some(record) = preferred_mx(&source.mx(domain)?)
    {
        if outgoing.is_empty() {
            outgoing.push(endpoint_from_mx(
                record.clone(),
                SMTP_SUBMISSION_PORT,
                SocketSecurity::StartTls,
            )?);
        }
        if incoming.is_empty() {
            incoming.push(endpoint_from_mx_as_imap(record)?);
        }
    }
    if incoming.is_empty() && outgoing.is_empty() {
        return Err(Error::Lookup(format!("no servers for {domain}")));
    }
    Ok(Autoconfig { incoming, outgoing })
}

/// IMAP on the MX host. MX does not name an IMAP server; this is the guess
/// used when SRV is also absent.
fn endpoint_from_mx_as_imap(record: MxRecord) -> Result<ServerEndpoint, Error> {
    let mut endpoint = endpoint_from_mx(record, IMAP_TLS_PORT, SocketSecurity::Tls)?;
    endpoint.protocol = crate::ServerProtocol::Imap;
    Ok(endpoint)
}

const SMTP_SUBMISSION_PORT: u16 = 587;
const IMAP_TLS_PORT: u16 = 993;

fn domain_of(email: &str) -> Result<&str, Error> {
    let Some((local, domain)) = email.split_once('@') else {
        return Err(Error::Lookup("address has no domain".into()));
    };
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return Err(Error::Lookup("address has no domain".into()));
    }
    Ok(domain)
}

fn first_srv(
    source: &impl ConfigSource,
    domain: &str,
    services: &[SrvService],
) -> Result<Vec<ServerEndpoint>, Error> {
    for service in services {
        let mut records = source.srv(*service, domain)?;
        if records.is_empty() {
            continue;
        }
        records.sort_by_key(|record| (record.priority, std::cmp::Reverse(record.weight)));
        let best = records.remove(0);
        return Ok(vec![endpoint_from_srv(*service, best)?]);
    }
    Ok(Vec::new())
}

fn preferred_mx(records: &[MxRecord]) -> Option<MxRecord> {
    records
        .iter()
        .min_by_key(|record| record.preference)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::discover;
    use crate::{
        ConfigSource, Error, MxRecord, ServerProtocol, SocketSecurity, SrvRecord, SrvService,
    };

    struct Fake {
        xml: Option<&'static [u8]>,
        srv: Vec<(SrvService, SrvRecord)>,
        mx: Vec<MxRecord>,
    }

    impl ConfigSource for Fake {
        fn ispdb_xml(&self, domain: &str) -> Result<Option<Vec<u8>>, Error> {
            assert_eq!(domain, "example.com");
            Ok(self.xml.map(<[u8]>::to_vec))
        }

        fn srv(&self, service: SrvService, domain: &str) -> Result<Vec<SrvRecord>, Error> {
            assert_eq!(domain, "example.com");
            Ok(self
                .srv
                .iter()
                .filter(|(kind, _)| *kind == service)
                .map(|(_, record)| record.clone())
                .collect())
        }

        fn mx(&self, domain: &str) -> Result<Vec<MxRecord>, Error> {
            assert_eq!(domain, "example.com");
            Ok(self.mx.clone())
        }
    }

    #[test]
    fn ispdb_xml_fills_host_port_and_security() {
        let source = Fake {
            xml: Some(include_bytes!("../fixtures/mozilla-autoconfig.xml")),
            srv: Vec::new(),
            mx: Vec::new(),
        };
        let config = discover(&source, "ada@example.com").unwrap();
        assert_eq!(config.incoming[0].host, "imap.example.com");
        assert_eq!(config.incoming[0].port, 993);
        assert_eq!(config.incoming[0].security, SocketSecurity::Tls);
        assert_eq!(config.outgoing[0].host, "smtp.example.com");
        assert_eq!(config.outgoing[0].port, 465);
        assert_eq!(config.outgoing[0].security, SocketSecurity::Tls);
    }

    #[test]
    fn srv_picks_the_lower_priority_and_sets_security() {
        let source = Fake {
            xml: None,
            srv: vec![
                (
                    SrvService::Imaps,
                    SrvRecord {
                        target: "imap-late.example.com".into(),
                        port: 993,
                        priority: 10,
                        weight: 0,
                    },
                ),
                (
                    SrvService::Imaps,
                    SrvRecord {
                        target: "imap.example.com".into(),
                        port: 993,
                        priority: 0,
                        weight: 5,
                    },
                ),
                (
                    SrvService::Submission,
                    SrvRecord {
                        target: "smtp.example.com".into(),
                        port: 587,
                        priority: 1,
                        weight: 0,
                    },
                ),
            ],
            mx: Vec::new(),
        };
        let config = discover(&source, "ada@example.com").unwrap();
        assert_eq!(config.incoming[0].host, "imap.example.com");
        assert_eq!(config.incoming[0].security, SocketSecurity::Tls);
        assert_eq!(config.incoming[0].protocol, ServerProtocol::Imap);
        assert_eq!(config.outgoing[0].host, "smtp.example.com");
        assert_eq!(config.outgoing[0].port, 587);
        assert_eq!(config.outgoing[0].security, SocketSecurity::StartTls);
    }

    #[test]
    fn mx_guesses_submission_and_imap_when_nothing_else_answered() {
        let source = Fake {
            xml: None,
            srv: Vec::new(),
            mx: vec![
                MxRecord {
                    exchange: "mx-backup.example.com".into(),
                    preference: 20,
                },
                MxRecord {
                    exchange: "mx.example.com".into(),
                    preference: 10,
                },
            ],
        };
        let config = discover(&source, "ada@example.com").unwrap();
        assert_eq!(config.incoming[0].host, "mx.example.com");
        assert_eq!(config.incoming[0].port, 993);
        assert_eq!(config.incoming[0].security, SocketSecurity::Tls);
        assert_eq!(config.incoming[0].protocol, ServerProtocol::Imap);
        assert_eq!(config.outgoing[0].host, "mx.example.com");
        assert_eq!(config.outgoing[0].port, 587);
        assert_eq!(config.outgoing[0].security, SocketSecurity::StartTls);
        assert_eq!(config.outgoing[0].protocol, ServerProtocol::Smtp);
    }

    #[test]
    fn a_bare_address_is_rejected() {
        let source = Fake {
            xml: None,
            srv: Vec::new(),
            mx: Vec::new(),
        };
        assert!(matches!(discover(&source, "ada"), Err(Error::Lookup(_))));
    }
}
