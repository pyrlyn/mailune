//! Mozilla ISP autoconfig XML and a `.well-known` JSON document.
//!
//! Both functions parse text. They do not resolve a name or fetch a URL.

use mailune_protocol::TransportSecurity;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Deserialize;

use crate::Error;

/// How the socket is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketSecurity {
    /// Implicit TLS. Mozilla calls this `SSL`.
    Tls,
    /// Cleartext handshake, then TLS. Mozilla calls this `STARTTLS`.
    StartTls,
    /// No encryption.
    Plain,
}

impl SocketSecurity {
    /// Envelope-level flag. STARTTLS still counts as TLS once the session is up.
    pub fn transport(self) -> TransportSecurity {
        match self {
            Self::Tls | Self::StartTls => TransportSecurity::Tls,
            Self::Plain => TransportSecurity::Clear,
        }
    }
}

/// Protocol the endpoint speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerProtocol {
    /// IMAP.
    Imap,
    /// SMTP submission.
    Smtp,
}

/// One server the caller can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEndpoint {
    /// IMAP or SMTP.
    pub protocol: ServerProtocol,
    /// Hostname. Not looked up here.
    pub host: String,
    /// TCP port from the document or the SRV record.
    pub port: u16,
    /// Socket protection.
    pub security: SocketSecurity,
}

/// Incoming and outgoing servers from one document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Autoconfig {
    /// IMAP endpoints, in document order.
    pub incoming: Vec<ServerEndpoint>,
    /// SMTP endpoints, in document order.
    pub outgoing: Vec<ServerEndpoint>,
}

/// One SRV answer. The caller performed the lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrvRecord {
    /// Target host from the record.
    pub target: String,
    /// Port from the record.
    pub port: u16,
    /// Priority. Lower is preferred. Stored for the caller; not sorted here.
    pub priority: u16,
    /// Weight within the same priority. Stored for the caller.
    pub weight: u16,
}

/// RFC 6186 service the caller queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrvService {
    /// `_imaps._tcp`. Implicit TLS.
    Imaps,
    /// `_imap._tcp`. STARTTLS.
    Imap,
    /// `_submissions._tcp`. Implicit TLS.
    Submissions,
    /// `_submission._tcp`. STARTTLS.
    Submission,
}

/// One MX answer. The caller performed the lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MxRecord {
    /// Exchange host.
    pub exchange: String,
    /// Preference. Lower is preferred. Stored for the caller.
    pub preference: u16,
}

/// Parses a Mozilla `clientConfig` XML document.
///
/// # Errors
///
/// Returns [`Error::Autoconfig`] when the XML is malformed or a server is
/// missing its host, port, socket type, or a known `type`.
pub fn parse_mozilla_xml(xml: &str) -> Result<Autoconfig, Error> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut config = Autoconfig::default();
    let mut side = Side::None;
    let mut draft: Option<Draft> = None;
    let mut field = Field::None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name().as_ref());
                match name.as_str() {
                    "incomingServer" => {
                        side = Side::Incoming;
                        draft = Some(Draft::from_type(
                            &attr(&event, "type")?,
                            ServerProtocol::Imap,
                        )?);
                        field = Field::None;
                    }
                    "outgoingServer" => {
                        side = Side::Outgoing;
                        draft = Some(Draft::from_type(
                            &attr(&event, "type")?,
                            ServerProtocol::Smtp,
                        )?);
                        field = Field::None;
                    }
                    "hostname" => field = Field::Host,
                    "port" => field = Field::Port,
                    "socketType" => field = Field::Security,
                    _ => {}
                }
            }
            Ok(Event::Text(event)) => {
                let text = event.xml10_content();
                let text = text.trim();
                if text.is_empty() {
                    buf.clear();
                    continue;
                }
                if let Some(draft) = draft.as_mut() {
                    match field {
                        Field::Host => draft.host = text.to_string(),
                        Field::Port => {
                            draft.port = Some(parse_port(text)?);
                        }
                        Field::Security => draft.security = Some(parse_socket(text)?),
                        Field::None => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let name = local_name(event.name().as_ref());
                match name.as_str() {
                    "hostname" | "port" | "socketType" => field = Field::None,
                    "incomingServer" | "outgoingServer" => {
                        let Some(draft) = draft.take() else {
                            return Err(Error::Autoconfig("server element was not opened".into()));
                        };
                        let endpoint = draft.finish()?;
                        match side {
                            Side::Incoming => config.incoming.push(endpoint),
                            Side::Outgoing => config.outgoing.push(endpoint),
                            Side::None => {
                                return Err(Error::Autoconfig("server element had no side".into()));
                            }
                        }
                        side = Side::None;
                        field = Field::None;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(Error::Autoconfig(err.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if config.incoming.is_empty() && config.outgoing.is_empty() {
        return Err(Error::Autoconfig("document has no servers".into()));
    }
    Ok(config)
}

/// Parses a `.well-known` JSON autoconfig document the caller already fetched.
///
/// The shape is `incoming` and `outgoing` arrays of `{protocol, host, port, security}`.
/// `security` is `tls`, `starttls`, or `plain`. Incoming entries are IMAP.
/// Outgoing entries are SMTP.
///
/// # Errors
///
/// Returns [`Error::Autoconfig`] when the JSON does not match that shape.
pub fn parse_well_known_json(json: &str) -> Result<Autoconfig, Error> {
    let doc: WellKnown =
        serde_json::from_str(json).map_err(|err| Error::Autoconfig(format!("json: {err}")))?;
    Ok(Autoconfig {
        incoming: servers(doc.incoming, ServerProtocol::Imap)?,
        outgoing: servers(doc.outgoing, ServerProtocol::Smtp)?,
    })
}

/// Turns one SRV answer into an endpoint. Security follows RFC 6186.
///
/// # Errors
///
/// Returns [`Error::Autoconfig`] when the target is empty or the port is 0.
pub fn endpoint_from_srv(service: SrvService, record: SrvRecord) -> Result<ServerEndpoint, Error> {
    let (protocol, security) = match service {
        SrvService::Imaps => (ServerProtocol::Imap, SocketSecurity::Tls),
        SrvService::Imap => (ServerProtocol::Imap, SocketSecurity::StartTls),
        SrvService::Submissions => (ServerProtocol::Smtp, SocketSecurity::Tls),
        SrvService::Submission => (ServerProtocol::Smtp, SocketSecurity::StartTls),
    };
    endpoint(protocol, record.target, record.port, security)
}

/// Turns one MX answer into an SMTP endpoint.
///
/// The caller chooses the port and the socket security. MX does not carry them.
///
/// # Errors
///
/// Returns [`Error::Autoconfig`] when the exchange is empty or the port is 0.
pub fn endpoint_from_mx(
    record: MxRecord,
    port: u16,
    security: SocketSecurity,
) -> Result<ServerEndpoint, Error> {
    endpoint(ServerProtocol::Smtp, record.exchange, port, security)
}

#[derive(Clone, Copy)]
enum Side {
    None,
    Incoming,
    Outgoing,
}

#[derive(Clone, Copy)]
enum Field {
    None,
    Host,
    Port,
    Security,
}

struct Draft {
    protocol: ServerProtocol,
    host: String,
    port: Option<u16>,
    security: Option<SocketSecurity>,
}

impl Draft {
    fn from_type(raw: &str, expected: ServerProtocol) -> Result<Self, Error> {
        let protocol = match raw {
            "imap" => ServerProtocol::Imap,
            "smtp" => ServerProtocol::Smtp,
            other => {
                return Err(Error::Autoconfig(format!("unknown server type {other}")));
            }
        };
        if protocol != expected {
            return Err(Error::Autoconfig(format!(
                "server type {raw} is on the wrong element"
            )));
        }
        Ok(Self {
            protocol,
            host: String::new(),
            port: None,
            security: None,
        })
    }

    fn finish(self) -> Result<ServerEndpoint, Error> {
        let Some(port) = self.port else {
            return Err(Error::Autoconfig("server is missing a port".into()));
        };
        let Some(security) = self.security else {
            return Err(Error::Autoconfig("server is missing a socket type".into()));
        };
        endpoint(self.protocol, self.host, port, security)
    }
}

fn endpoint(
    protocol: ServerProtocol,
    host: String,
    port: u16,
    security: SocketSecurity,
) -> Result<ServerEndpoint, Error> {
    let host = host.trim().to_string();
    if host.is_empty() || host == "." {
        return Err(Error::Autoconfig("server host is empty".into()));
    }
    if port == 0 {
        return Err(Error::Autoconfig("server port is 0".into()));
    }
    Ok(ServerEndpoint {
        protocol,
        host,
        port,
        security,
    })
}

fn local_name(name: &str) -> String {
    match name.rfind(':') {
        Some(index) => name[index + 1..].to_string(),
        None => name.to_string(),
    }
}

fn attr(event: &quick_xml::events::BytesStart<'_>, key: &str) -> Result<String, Error> {
    let Some(value) = event
        .try_get_attribute(key)
        .map_err(|err| Error::Autoconfig(err.to_string()))?
    else {
        return Err(Error::Autoconfig(format!("missing {key} attribute")));
    };
    value
        .normalized_value(quick_xml::XmlVersion::Explicit1_0)
        .map(|text| text.into_owned())
        .map_err(|err| Error::Autoconfig(err.to_string()))
}

fn parse_port(text: &str) -> Result<u16, Error> {
    text.parse::<u16>()
        .map_err(|_| Error::Autoconfig(format!("port {text} is not a number")))
}

fn parse_socket(text: &str) -> Result<SocketSecurity, Error> {
    match text.to_ascii_lowercase().as_str() {
        "ssl" | "tls" => Ok(SocketSecurity::Tls),
        "starttls" => Ok(SocketSecurity::StartTls),
        "plain" => Ok(SocketSecurity::Plain),
        other => Err(Error::Autoconfig(format!("unknown socket type {other}"))),
    }
}

#[derive(Deserialize)]
struct WellKnown {
    incoming: Vec<JsonServer>,
    outgoing: Vec<JsonServer>,
}

#[derive(Deserialize)]
struct JsonServer {
    protocol: String,
    host: String,
    port: u16,
    security: String,
}

fn servers(rows: Vec<JsonServer>, expected: ServerProtocol) -> Result<Vec<ServerEndpoint>, Error> {
    let expected_name = match expected {
        ServerProtocol::Imap => "imap",
        ServerProtocol::Smtp => "smtp",
    };
    rows.into_iter()
        .map(|row| {
            if row.protocol != expected_name {
                return Err(Error::Autoconfig(format!(
                    "protocol {} is not {expected_name}",
                    row.protocol
                )));
            }
            endpoint(expected, row.host, row.port, parse_socket(&row.security)?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        MxRecord, ServerProtocol, SocketSecurity, SrvRecord, SrvService, endpoint_from_mx,
        endpoint_from_srv, parse_mozilla_xml, parse_well_known_json,
    };
    use crate::Error;
    use mailune_protocol::TransportSecurity;

    #[test]
    fn mozilla_fixture_yields_host_port_and_security() {
        let config = parse_mozilla_xml(include_str!("../fixtures/mozilla-autoconfig.xml")).unwrap();
        assert_eq!(config.incoming.len(), 1);
        let imap = &config.incoming[0];
        assert_eq!(imap.protocol, ServerProtocol::Imap);
        assert_eq!(imap.host, "imap.example.com");
        assert_eq!(imap.port, 993);
        assert_eq!(imap.security, SocketSecurity::Tls);
        assert_eq!(imap.security.transport(), TransportSecurity::Tls);
        let smtp = &config.outgoing[0];
        assert_eq!(smtp.host, "smtp.example.com");
        assert_eq!(smtp.port, 465);
        assert_eq!(smtp.security, SocketSecurity::Tls);
    }

    #[test]
    fn well_known_json_fixture_yields_starttls() {
        let config = parse_well_known_json(include_str!("../fixtures/well-known.json")).unwrap();
        let smtp = &config.outgoing[0];
        assert_eq!(smtp.protocol, ServerProtocol::Smtp);
        assert_eq!(smtp.host, "smtp.example.com");
        assert_eq!(smtp.port, 587);
        assert_eq!(smtp.security, SocketSecurity::StartTls);
        assert_eq!(smtp.security.transport(), TransportSecurity::Tls);
    }

    #[test]
    fn srv_record_sets_security_from_the_service_name() {
        let record = SrvRecord {
            target: "imap.example.com".into(),
            port: 993,
            priority: 0,
            weight: 1,
        };
        let endpoint = endpoint_from_srv(SrvService::Imaps, record).unwrap();
        assert_eq!(endpoint.security, SocketSecurity::Tls);
        assert_eq!(endpoint.protocol, ServerProtocol::Imap);

        let submission = endpoint_from_srv(
            SrvService::Submission,
            SrvRecord {
                target: "smtp.example.com".into(),
                port: 587,
                priority: 0,
                weight: 0,
            },
        )
        .unwrap();
        assert_eq!(submission.security, SocketSecurity::StartTls);
        assert_eq!(submission.protocol, ServerProtocol::Smtp);
    }

    #[test]
    fn mx_record_uses_the_port_the_caller_supplies() {
        let endpoint = endpoint_from_mx(
            MxRecord {
                exchange: "mx.example.com".into(),
                preference: 10,
            },
            25,
            SocketSecurity::StartTls,
        )
        .unwrap();
        assert_eq!(endpoint.host, "mx.example.com");
        assert_eq!(endpoint.port, 25);
        assert_eq!(endpoint.protocol, ServerProtocol::Smtp);
    }

    #[test]
    fn empty_srv_target_is_rejected() {
        let err = endpoint_from_srv(
            SrvService::Imap,
            SrvRecord {
                target: " ".into(),
                port: 143,
                priority: 0,
                weight: 0,
            },
        );
        assert!(matches!(err, Err(Error::Autoconfig(_))));
    }
}
