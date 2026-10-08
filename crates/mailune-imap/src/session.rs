//! IMAP session states over a byte stream.
//!
//! [`MemStream`] is the scripted server behind [`std::io::Read`] and
//! [`std::io::Write`]. [`Config::tls_required`] is only a flag: a later
//! socket would refuse cleartext, and this session never handshakes.

use std::fmt;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::Error;
use crate::script::Scripted;

/// How to sign in, and whether a real socket must use TLS.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    /// A later connection must be TLS. This session does not perform a handshake.
    pub tls_required: bool,
    /// IMAP LOGIN username.
    pub username: String,
    /// IMAP LOGIN password. [`Debug`] prints `redacted`.
    pub password: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field("tls_required", &self.tls_required)
            .field("username", &self.username)
            .field("password", &"redacted")
            .finish()
    }
}

/// Where the session is in the IMAP state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    NotAuthenticated,
    Authenticated,
}

/// CAPABILITY and LOGIN against whatever stream the caller provides.
#[derive(Debug)]
pub struct Connection<S> {
    io: S,
    config: Config,
    buf: Vec<u8>,
    tag: u32,
    phase: Phase,
    capabilities: Vec<String>,
}

impl Connection<MemStream> {
    /// Read the greeting from a new in-memory server.
    pub fn open(config: Config) -> Result<Self, Error> {
        Self::over(MemStream::new(Scripted::new()), config)
    }
}

impl<S: Read + Write> Connection<S> {
    /// Read the greeting from `io`.
    pub fn over(io: S, config: Config) -> Result<Self, Error> {
        let mut connection = Self {
            io,
            config,
            buf: Vec::new(),
            tag: 0,
            phase: Phase::NotAuthenticated,
            capabilities: Vec::new(),
        };
        connection.read_greeting()?;
        Ok(connection)
    }

    /// Whether the last CAPABILITY reply listed `name`.
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities
            .iter()
            .any(|item| item.eq_ignore_ascii_case(name))
    }

    /// Sends `body` under a new tag and returns every line up to the tagged
    /// reply.
    ///
    /// # Errors
    ///
    /// [`Error::Rejected`] on NO or BAD, [`Error::Session`] when the stream stops.
    pub(crate) fn run(&mut self, body: &str) -> Result<String, Error> {
        let tag = self.send(body)?;
        self.finish(&tag)
    }

    /// Whether a real socket would be required to use TLS. No handshake runs.
    pub fn tls_required(&self) -> bool {
        self.config.tls_required
    }

    /// True after LOGIN was accepted.
    pub fn is_authenticated(&self) -> bool {
        self.phase == Phase::Authenticated
    }

    /// CAPABILITY, from the not-authenticated state.
    pub fn capability(&mut self) -> Result<Vec<String>, Error> {
        let tag = self.next_tag();
        self.command(&tag, "CAPABILITY")?;
        let reply = self.read_until_tag(&tag)?;
        if !tagged_ok(&reply, &tag) {
            return Err(Error::Rejected);
        }
        self.capabilities = capability_names(&reply);
        Ok(self.capabilities.clone())
    }

    /// LOGIN with the username and password from [`Config`].
    pub fn login(&mut self) -> Result<(), Error> {
        let tag = self.next_tag();
        let user = quote_atom(&self.config.username);
        let pass = quote_atom(&self.config.password);
        self.command(&tag, &format!("LOGIN {user} {pass}"))?;
        let reply = self.read_until_tag(&tag)?;
        if !tagged_ok(&reply, &tag) {
            return Err(Error::Rejected);
        }
        self.phase = Phase::Authenticated;
        Ok(())
    }

    /// Sends `body` under a new tag and returns the tag, without waiting.
    pub(crate) fn send(&mut self, body: &str) -> Result<String, Error> {
        let tag = self.next_tag();
        self.command(&tag, body)?;
        Ok(tag)
    }

    /// Sends one untagged line, such as `DONE`.
    pub(crate) fn send_line(&mut self, line: &str) -> Result<(), Error> {
        self.io
            .write_all(format!("{line}\r\n").as_bytes())
            .map_err(|_| Error::Session)
    }

    /// Reads one line, waiting for it.
    pub(crate) fn read_line(&mut self) -> Result<String, Error> {
        self.read_one_line()
    }

    /// Reads the tagged reply to `tag`, failing on NO or BAD.
    pub(crate) fn finish(&mut self, tag: &str) -> Result<String, Error> {
        let reply = self.read_until_tag(tag)?;
        if tagged_ok(&reply, tag) {
            Ok(reply)
        } else {
            Err(Error::Rejected)
        }
    }

    /// Every complete line the stream has ready, without waiting for more.
    /// A `WouldBlock` read ends the batch; any other failure or an end of
    /// stream is [`Error::Session`].
    pub(crate) fn read_available(&mut self) -> Result<Vec<String>, Error> {
        let mut tmp = [0u8; 512];
        loop {
            match self.io.read(&mut tmp) {
                Ok(0) => return Err(Error::Session),
                Ok(read) => self.buf.extend_from_slice(&tmp[..read]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => return Err(Error::Session),
            }
        }
        let mut lines = Vec::new();
        while let Some(split) = self.buf.windows(2).position(|window| window == b"\r\n") {
            let line: Vec<u8> = self.buf.drain(..=split + 1).collect();
            lines.push(String::from_utf8_lossy(&line[..line.len().saturating_sub(2)]).into_owned());
        }
        Ok(lines)
    }

    fn next_tag(&mut self) -> String {
        self.tag += 1;
        format!("A{}", self.tag)
    }

    fn command(&mut self, tag: &str, body: &str) -> Result<(), Error> {
        let line = format!("{tag} {body}\r\n");
        self.io
            .write_all(line.as_bytes())
            .map_err(|_| Error::Session)?;
        Ok(())
    }

    fn read_greeting(&mut self) -> Result<(), Error> {
        let reply = self.read_one_line()?;
        if reply.starts_with("* OK") || reply.starts_with("* PREAUTH") {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }

    fn read_until_tag(&mut self, tag: &str) -> Result<String, Error> {
        let mut collected = String::new();
        loop {
            let line = self.read_one_line()?;
            let done = line.starts_with(tag) && line[tag.len()..].starts_with(' ');
            collected.push_str(&line);
            collected.push_str("\r\n");
            if done {
                return Ok(collected);
            }
        }
    }

    fn read_one_line(&mut self) -> Result<String, Error> {
        loop {
            if let Some(split) = self.buf.windows(2).position(|window| window == b"\r\n") {
                let line: Vec<u8> = self.buf.drain(..=split + 1).collect();
                let text = String::from_utf8_lossy(&line[..line.len().saturating_sub(2)]);
                return Ok(text.into_owned());
            }
            let mut tmp = [0u8; 512];
            let read = self.io.read(&mut tmp).map_err(|_| Error::Session)?;
            if read == 0 {
                return Err(Error::Session);
            }
            self.buf.extend_from_slice(&tmp[..read]);
        }
    }
}

/// A scripted server that successive connections share, so a reconnect
/// finds the same mailboxes.
#[derive(Debug, Clone)]
pub struct SharedServer(Arc<Mutex<Scripted>>);

impl SharedServer {
    /// Shares `server`.
    pub fn new(server: Scripted) -> Self {
        Self(Arc::new(Mutex::new(server)))
    }

    /// Opens a new connection. An earlier one stops working.
    pub fn connect(&self) -> MemStream {
        let (generation, greeting) = match self.0.lock() {
            Ok(mut server) => server.reconnect(),
            Err(_) => (0, ""),
        };
        MemStream {
            server: Arc::clone(&self.0),
            inbound: greeting.as_bytes().to_vec(),
            generation,
        }
    }

    /// Runs `change` on the server between or during sessions. `None` when
    /// a panicking test poisoned it.
    pub fn with<T>(&self, change: impl FnOnce(&mut Scripted) -> T) -> Option<T> {
        self.0.lock().ok().map(|mut server| change(&mut server))
    }
}

/// The scripted server as a byte stream. Both sides live in this process.
///
/// An empty stream reads as `WouldBlock`, like a socket with nothing to say
/// yet; a dropped connection reads and writes as `ConnectionReset`.
#[derive(Debug)]
pub struct MemStream {
    server: Arc<Mutex<Scripted>>,
    inbound: Vec<u8>,
    generation: u64,
}

impl MemStream {
    /// A stream to `server`, with its greeting ready to read.
    pub fn new(server: Scripted) -> Self {
        SharedServer::new(server).connect()
    }

    fn server(&self) -> io::Result<MutexGuard<'_, Scripted>> {
        let server = self
            .server
            .lock()
            .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
        if server.alive(self.generation) {
            Ok(server)
        } else {
            Err(io::Error::from(io::ErrorKind::ConnectionReset))
        }
    }
}

impl Read for MemStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.inbound.is_empty() {
            let pushed = self.server()?.take_pushed();
            self.inbound = pushed;
        }
        if self.inbound.is_empty() {
            return Err(io::Error::from(io::ErrorKind::WouldBlock));
        }
        let count = buf.len().min(self.inbound.len());
        buf[..count].copy_from_slice(&self.inbound[..count]);
        self.inbound.drain(..count);
        Ok(count)
    }
}

impl Write for MemStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let reply = self.server()?.ingest(buf);
        self.inbound.extend_from_slice(&reply);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn tagged_ok(reply: &str, tag: &str) -> bool {
    reply.lines().any(|line| {
        let rest = line.trim_end_matches(['\r', '\n']);
        rest.starts_with(tag) && rest[tag.len()..].trim_start().starts_with("OK")
    })
}

fn capability_names(reply: &str) -> Vec<String> {
    reply
        .lines()
        .find(|line| {
            line.trim_start()
                .to_ascii_uppercase()
                .starts_with("* CAPABILITY")
        })
        .map(|line| {
            line.split_whitespace()
                .skip(2)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn quote_atom(value: &str) -> String {
    let plain = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'@' | b'.' | b'-' | b'_'));
    if plain {
        value.to_string()
    } else {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Connection};

    #[test]
    fn capability_and_login_use_the_in_memory_stream() {
        let config = Config {
            tls_required: true,
            username: "ana".to_string(),
            password: "secret".to_string(),
        };
        assert_eq!(
            format!("{config:?}"),
            "Config { tls_required: true, username: \"ana\", password: \"redacted\" }"
        );
        let mut session = Connection::open(config).unwrap();
        assert!(session.tls_required());
        assert!(!session.is_authenticated());
        let caps = session.capability().unwrap();
        assert!(caps.iter().any(|item| item == "X-GM-EXT-1"));
        assert!(caps.iter().any(|item| item == "IMAP4rev1"));
        session.login().unwrap();
        assert!(session.is_authenticated());
    }
}
