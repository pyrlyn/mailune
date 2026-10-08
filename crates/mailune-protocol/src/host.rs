//! Effects the core asks the platform for: network, clock, files, and the
//! host callbacks.
//!
//! Separate from the records ([`crate::Submission`], [`crate::Event`]) because
//! those are data and these are the only way out of the process. Nothing here
//! opens a socket, a file, or a process.
//!
//! Methods that wait return `impl Future + Send`: the desugaring of `async fn`
//! that can name `Send`. The `async fn` sugar cannot state that bound, and
//! this crate denies the lint that would hide it, because a future that is
//! not `Send` cannot be spawned. Implementations still write `async fn`.
//! Methods that only read a value the host already has, or that must return
//! without blocking the caller, stay synchronous: the host hops to its own
//! thread. These futures are not dyn-compatible, so callers are generic over
//! the trait. This crate does not depend on `async-trait`; a foreign `dyn`
//! boundary does not exist yet.

use std::fmt;
use std::future::Future;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::Error;
use crate::ids::AccountId;

/// A TCP connection the host opened. Meaningful only to that host.
///
/// The number is the host's. Another crate cannot see the field, so it mints
/// an id with [`ConnId::from_raw`] and reads it back with [`ConnId::as_raw`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(u64);

impl ConnId {
    /// An id this host minted. The contract does not interpret the number.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The host's number, so it can match the id back to its socket.
    pub fn as_raw(self) -> u64 {
        self.0
    }
}

/// Cleartext TCP.
///
/// TLS stays with the protocol adapter. This trait does not negotiate it, so
/// a test can answer without certificates, and the trait does not pull a TLS
/// stack into the contract.
pub trait Net: Send + Sync {
    /// Opens `host`:`port`. `host` is a name or address the caller already chose.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the connection cannot be opened. The message must
    /// not contain secret material.
    fn connect(&self, host: &str, port: u16) -> impl Future<Output = Result<ConnId, Error>> + Send;

    /// Copies the next bytes into `buf`.
    ///
    /// `Ok(0)` with a non-empty `buf` means the peer closed. An empty `buf`
    /// returns `Ok(0)` and does not mean the peer closed.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when `id` is not an open connection.
    fn read(&self, id: ConnId, buf: &mut [u8])
    -> impl Future<Output = Result<usize, Error>> + Send;

    /// Sends `buf` and returns how many bytes the host accepted.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when `id` is not an open connection.
    fn write(&self, id: ConnId, buf: &[u8]) -> impl Future<Output = Result<usize, Error>> + Send;

    /// Closes `id`.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when `id` is already closed or was never opened, so a
    /// stale id is not mistaken for a live pipe.
    fn close(&self, id: ConnId) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Wall time and waits.
///
/// `now` is synchronous: it is a value the host already has. `sleep` is async
/// so a fake can advance time without blocking the thread, and a real host
/// can wait without a blocking sleep in the core.
pub trait Clock: Send + Sync {
    /// The time the core should treat as now.
    fn now(&self) -> SystemTime;

    /// Waits until `duration` has passed on this clock.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}

/// Whole-file reads and writes.
///
/// Paths are opaque. The host decides which of them exist; the trait does not
/// interpret `..`. Streaming a large body is [`Net`]'s job. This trait is for
/// the small files the core names directly.
pub trait Fs: Send + Sync {
    /// Bytes at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] when the host has no such path. [`Error::Host`]
    /// when the host cannot read it.
    fn read(&self, path: &Path) -> impl Future<Output = Result<Vec<u8>, Error>> + Send;

    /// Replaces the bytes at `path`, creating it when it was absent.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the host cannot write.
    fn write(&self, path: &Path, bytes: &[u8]) -> impl Future<Output = Result<(), Error>> + Send;

    /// Removes `path`. An absent path is success, so a retry does not fail.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the host cannot remove it.
    fn remove(&self, path: &Path) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Which secret [`SecretId`] names.
///
/// The three kinds the store is for: tokens, passwords, and the database key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretKind {
    /// OAuth or other bearer token.
    Token,
    /// IMAP or SMTP password.
    Password,
    /// SQLCipher key for the local database.
    DbKey,
}

/// Names a secret without containing one.
///
/// A database key is not per account, so [`SecretId::db_key`] has no account.
/// Token and password always name one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretId {
    account: Option<AccountId>,
    kind: SecretKind,
}

impl SecretId {
    /// Bearer token stored for `account`.
    pub fn token(account: AccountId) -> Self {
        Self {
            account: Some(account),
            kind: SecretKind::Token,
        }
    }

    /// Password stored for `account`.
    pub fn password(account: AccountId) -> Self {
        Self {
            account: Some(account),
            kind: SecretKind::Password,
        }
    }

    /// Database key. There is one, shared by the local store.
    pub fn db_key() -> Self {
        Self {
            account: None,
            kind: SecretKind::DbKey,
        }
    }

    /// Which kind of secret this id names.
    pub fn kind(&self) -> SecretKind {
        self.kind
    }

    /// Account, when the secret belongs to one. `None` for [`SecretId::db_key`].
    pub fn account(&self) -> Option<&AccountId> {
        self.account.as_ref()
    }
}

/// Bytes a [`SecretStore`] holds.
///
/// `Debug` prints `Secret(redacted)` and there is no `Display`, so formatting
/// the value cannot leak it. The only read is [`Secret::as_bytes`].
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
    bytes: Vec<u8>,
}

impl Secret {
    /// Copies `bytes` into the secret. The copy is what the store keeps.
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        Self {
            bytes: bytes.as_ref().to_vec(),
        }
    }

    /// The raw bytes. Do not log them.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(redacted)")
    }
}

/// Tokens, passwords, and the database key.
///
/// The host talks to the keychain (Android, through the same callback). This
/// crate does not. Implementations must not log the bytes they store or return.
pub trait SecretStore: Send + Sync {
    /// The secret for `id`, or `None` when nothing is stored.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the keychain cannot be read. A missing secret is
    /// `Ok(None)`, not an error.
    fn get(&self, id: &SecretId) -> impl Future<Output = Result<Option<Secret>, Error>> + Send;

    /// Stores `secret` for `id`, replacing a previous value.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the keychain cannot be written.
    fn put(&self, id: &SecretId, secret: &Secret)
    -> impl Future<Output = Result<(), Error>> + Send;

    /// Forgets `id`. An absent id is success.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the keychain cannot be updated.
    fn delete(&self, id: &SecretId) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Text the operating system may show outside the app.
///
/// Not [`crate::Event::Notice`]: that is an in-app sentence. This is a banner.
/// It is not a place for a token or a password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// Account the banner is about.
    pub account: AccountId,
    /// Short title.
    pub title: String,
    /// Preview the person may see. Not a credential.
    pub body: String,
}

/// Posts a banner and the app badge.
///
/// Synchronous because the caller must not wait: the host hops to its UI
/// thread and returns. Mail state does not roll back when a banner fails.
pub trait Notifier: Send + Sync {
    /// Shows `notice`.
    fn notify(&self, notice: &Notification);

    /// Sets the app badge. `0` clears it.
    fn set_badge(&self, count: u32);
}

/// What kind of path the device has. Body fetch follows this.
///
/// Unknown is [`NetworkPath::Offline`]: the core does not download bodies on
/// a path it has not seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPath {
    /// No usable path, or the host has not reported one yet.
    Offline,
    /// Wi-Fi or ethernet.
    Unmetered,
    /// Cellular, or a path the OS marks expensive.
    Metered,
}

/// Last path the host observed.
///
/// Synchronous: the monitor already has the answer, and the decision to fetch
/// must not wait on a callback.
pub trait NetworkState: Send + Sync {
    /// The current path.
    fn path(&self) -> NetworkPath;
}

/// What the host needs to present an authorization session.
///
/// The code verifier stays with the caller. It is a secret and is not a field
/// here; `authorization_url` carries only the challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRequest {
    /// Account the person is signing in.
    pub account: AccountId,
    /// Authorization URL, including the PKCE challenge.
    pub authorization_url: String,
    /// Redirect the host should catch: a custom scheme or a loopback URL.
    pub callback: String,
}

/// How the sign-in sheet ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    /// Callback URL. It carries the authorization code, so it is a [`Secret`]
    /// and its `Debug` stays redacted.
    Completed(Secret),
    /// The person dismissed the sheet. Not a failure.
    Cancelled,
}

/// System sign-in sheet. The core cannot present one itself.
///
/// Async because it waits for the person.
pub trait AuthSession: Send + Sync {
    /// Presents `request` and waits for the person.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the sheet cannot be presented. Dismissal is
    /// [`AuthResult::Cancelled`], not an error.
    fn authenticate(
        &self,
        request: &AuthRequest,
    ) -> impl Future<Output = Result<AuthResult, Error>> + Send;
}

/// One on-device model the host can run.
///
/// Cloud models are a different adapter. This value stays on the device, so
/// the callback is not a path that sends mail to a cloud model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCapability {
    /// The host's name for the model.
    pub id: String,
    /// Context window, when the host knows it.
    pub context_tokens: Option<u32>,
}

/// A completion request.
///
/// `text` may contain mail. `Debug` redacts it so a log of the request is not
/// a copy of the message.
#[derive(Clone, PartialEq, Eq)]
pub struct ModelPrompt {
    /// Text to complete. May contain mail. Do not log it.
    pub text: String,
    /// Upper bound the host should ask the model for.
    pub max_output_tokens: u32,
}

impl fmt::Debug for ModelPrompt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelPrompt")
            .field("text", &"redacted")
            .field("max_output_tokens", &self.max_output_tokens)
            .finish()
    }
}

/// On-device generation.
///
/// [`PlatformModel::capabilities`] is synchronous: the host already knows what
/// is installed. [`PlatformModel::complete`] waits for generation.
pub trait PlatformModel: Send + Sync {
    /// Installed models. Empty means the platform has none.
    fn capabilities(&self) -> Vec<ModelCapability>;

    /// Runs `prompt` on the platform model.
    ///
    /// The returned text is model output and untrusted.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when nothing is installed or generation fails. The
    /// message must not quote `prompt`.
    fn complete(&self, prompt: &ModelPrompt) -> impl Future<Output = Result<String, Error>> + Send;
}

/// Work the OS should run later, without the app in the foreground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackgroundWork {
    /// Refresh the visible mailbox and inbox headers.
    Sync,
    /// Fetch bodies. The host should wait for [`NetworkPath::Unmetered`].
    Bodies,
}

/// Registers work with the OS scheduler.
///
/// Synchronous because registration returns at once. The OS runs the task
/// later; this trait does not run it.
pub trait BackgroundScheduler: Send + Sync {
    /// Registers `work` for `account`, replacing a previous registration of
    /// the same kind.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the OS refuses the registration.
    fn schedule(&self, account: &AccountId, work: BackgroundWork) -> Result<(), Error>;

    /// Drops a previous registration. An absent one is success.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when the OS cannot update the registration.
    fn cancel(&self, account: &AccountId, work: BackgroundWork) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::pin;
    use std::sync::Mutex;
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use super::{
        AccountId, AuthRequest, AuthResult, AuthSession, BackgroundScheduler, BackgroundWork,
        Clock, Fs, ModelCapability, ModelPrompt, Net, NetworkPath, NetworkState, Notification,
        Notifier, PlatformModel, Secret, SecretId, SecretKind, SecretStore,
    };

    fn drive<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("fake host future waited"),
        }
    }

    struct Pipe {
        pending: VecDeque<u8>,
        written: Vec<u8>,
    }

    struct State {
        now: std::time::SystemTime,
        files: HashMap<PathBuf, Vec<u8>>,
        secrets: HashMap<SecretId, Secret>,
        notices: Vec<Notification>,
        badge: u32,
        path: NetworkPath,
        auth: AuthResult,
        seen_auth: Option<AuthRequest>,
        models: Vec<ModelCapability>,
        reply: String,
        jobs: Vec<(AccountId, BackgroundWork)>,
        next_conn: u64,
        dials: Vec<(String, u16)>,
        conns: HashMap<u64, Pipe>,
    }

    /// In-memory stand-in. It never touches the keychain, the disk, or the network.
    struct Fake {
        state: Mutex<State>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                state: Mutex::new(State {
                    now: std::time::SystemTime::UNIX_EPOCH,
                    files: HashMap::new(),
                    secrets: HashMap::new(),
                    notices: Vec::new(),
                    badge: 0,
                    path: NetworkPath::Offline,
                    auth: AuthResult::Cancelled,
                    seen_auth: None,
                    models: Vec::new(),
                    reply: String::new(),
                    jobs: Vec::new(),
                    next_conn: 1,
                    dials: Vec::new(),
                    conns: HashMap::new(),
                }),
            }
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, State> {
            self.state.lock().unwrap()
        }
    }

    fn unknown(operation: &'static str) -> crate::Error {
        crate::Error::host(operation, "unknown connection")
    }

    impl Net for Fake {
        async fn connect(&self, host: &str, port: u16) -> Result<super::ConnId, crate::Error> {
            let mut state = self.lock();
            state.dials.push((host.to_string(), port));
            let id = state.next_conn;
            state.next_conn += 1;
            state.conns.insert(
                id,
                Pipe {
                    pending: b"READY".iter().copied().collect(),
                    written: Vec::new(),
                },
            );
            Ok(super::ConnId::from_raw(id))
        }

        async fn read(&self, id: super::ConnId, buf: &mut [u8]) -> Result<usize, crate::Error> {
            let mut state = self.lock();
            let pipe = state
                .conns
                .get_mut(&id.as_raw())
                .ok_or_else(|| unknown("net.read"))?;
            let mut count = 0;
            if buf.is_empty() {
                return Ok(0);
            }
            for byte in &mut *buf {
                match pipe.pending.pop_front() {
                    Some(value) => {
                        *byte = value;
                        count += 1;
                    }
                    None => break,
                }
            }
            Ok(count)
        }

        async fn write(&self, id: super::ConnId, buf: &[u8]) -> Result<usize, crate::Error> {
            let mut state = self.lock();
            let pipe = state
                .conns
                .get_mut(&id.as_raw())
                .ok_or_else(|| unknown("net.write"))?;
            pipe.written.extend_from_slice(buf);
            Ok(buf.len())
        }

        async fn close(&self, id: super::ConnId) -> Result<(), crate::Error> {
            let mut state = self.lock();
            if state.conns.remove(&id.as_raw()).is_none() {
                return Err(unknown("net.close"));
            }
            Ok(())
        }
    }

    impl Clock for Fake {
        fn now(&self) -> std::time::SystemTime {
            self.lock().now
        }

        async fn sleep(&self, duration: Duration) {
            let mut state = self.lock();
            state.now = state.now.checked_add(duration).unwrap_or(state.now);
        }
    }

    impl Fs for Fake {
        async fn read(&self, path: &std::path::Path) -> Result<Vec<u8>, crate::Error> {
            let state = self.lock();
            state
                .files
                .get(path)
                .cloned()
                .ok_or_else(|| crate::Error::NotFound {
                    path: path.display().to_string(),
                })
        }

        async fn write(&self, path: &std::path::Path, bytes: &[u8]) -> Result<(), crate::Error> {
            self.lock().files.insert(path.to_path_buf(), bytes.to_vec());
            Ok(())
        }

        async fn remove(&self, path: &std::path::Path) -> Result<(), crate::Error> {
            let mut state = self.lock();
            let _ = state.files.remove(path);
            Ok(())
        }
    }

    impl SecretStore for Fake {
        async fn get(&self, id: &SecretId) -> Result<Option<Secret>, crate::Error> {
            Ok(self.lock().secrets.get(id).cloned())
        }

        async fn put(&self, id: &SecretId, secret: &Secret) -> Result<(), crate::Error> {
            self.lock().secrets.insert(id.clone(), secret.clone());
            Ok(())
        }

        async fn delete(&self, id: &SecretId) -> Result<(), crate::Error> {
            let mut state = self.lock();
            let _ = state.secrets.remove(id);
            Ok(())
        }
    }

    impl Notifier for Fake {
        fn notify(&self, notice: &Notification) {
            self.lock().notices.push(notice.clone());
        }

        fn set_badge(&self, count: u32) {
            self.lock().badge = count;
        }
    }

    impl NetworkState for Fake {
        fn path(&self) -> NetworkPath {
            self.lock().path
        }
    }

    impl AuthSession for Fake {
        async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, crate::Error> {
            let mut state = self.lock();
            state.seen_auth = Some(request.clone());
            Ok(state.auth.clone())
        }
    }

    impl PlatformModel for Fake {
        fn capabilities(&self) -> Vec<ModelCapability> {
            self.lock().models.clone()
        }

        async fn complete(&self, _prompt: &ModelPrompt) -> Result<String, crate::Error> {
            let state = self.lock();
            if state.models.is_empty() {
                return Err(crate::Error::host("platform.complete", "no platform model"));
            }
            Ok(state.reply.clone())
        }
    }

    impl BackgroundScheduler for Fake {
        fn schedule(&self, account: &AccountId, work: BackgroundWork) -> Result<(), crate::Error> {
            let mut state = self.lock();
            state
                .jobs
                .retain(|(id, kind)| !(id == account && *kind == work));
            state.jobs.push((account.clone(), work));
            Ok(())
        }

        fn cancel(&self, account: &AccountId, work: BackgroundWork) -> Result<(), crate::Error> {
            self.lock()
                .jobs
                .retain(|(id, kind)| !(id == account && *kind == work));
            Ok(())
        }
    }

    #[test]
    fn a_secret_debug_does_not_contain_the_bytes() {
        let secret = Secret::new(b"db-key-value");
        let rendered = format!("{secret:?}");
        assert!(!rendered.contains("db-key-value"), "{rendered}");
        assert_eq!(rendered, "Secret(redacted)");
        assert_eq!(secret.as_bytes(), b"db-key-value");
    }

    #[test]
    fn a_prompt_debug_does_not_contain_the_text() {
        let prompt = ModelPrompt {
            text: "mail body".into(),
            max_output_tokens: 16,
        };
        let rendered = format!("{prompt:?}");
        assert!(!rendered.contains("mail body"), "{rendered}");
        assert!(rendered.contains("redacted"), "{rendered}");
    }

    #[test]
    fn the_fake_implements_net() {
        let fake = Fake::new();
        let id = drive(fake.connect("imap.example", 993)).unwrap();
        let mut buf = [0u8; 8];
        let count = drive(Net::read(&fake, id, &mut buf)).unwrap();
        assert_eq!(&buf[..count], b"READY");
        assert_eq!(
            drive(Net::write(&fake, id, b"a001 NOOP")).unwrap(),
            b"a001 NOOP".len()
        );
        assert_eq!(fake.lock().conns[&id.as_raw()].written, b"a001 NOOP");
        assert_eq!(fake.lock().dials, [("imap.example".to_string(), 993)]);
        drive(fake.close(id)).unwrap();
        assert!(drive(fake.close(id)).is_err());
    }

    #[test]
    fn the_fake_implements_clock() {
        let fake = Fake::new();
        let start = fake.now();
        drive(fake.sleep(Duration::from_secs(5)));
        assert_eq!(
            fake.now().duration_since(start).unwrap(),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn the_fake_implements_fs() {
        let fake = Fake::new();
        let path = PathBuf::from("mail/draft");
        drive(Fs::write(&fake, &path, b"hello")).unwrap();
        assert_eq!(drive(Fs::read(&fake, &path)).unwrap(), b"hello");
        drive(fake.remove(&path)).unwrap();
        assert!(matches!(
            drive(Fs::read(&fake, &path)),
            Err(crate::Error::NotFound { .. })
        ));
        drive(fake.remove(&path)).unwrap();
    }

    #[test]
    fn the_fake_implements_secret_store() {
        let fake = Fake::new();
        let account = AccountId::new("a");
        let token = SecretId::token(account.clone());
        let password = SecretId::password(account);
        let db = SecretId::db_key();
        assert_eq!(token.kind(), SecretKind::Token);
        assert!(token.account().is_some());
        assert_eq!(db.kind(), SecretKind::DbKey);
        assert!(db.account().is_none());

        drive(fake.put(&token, &Secret::new(b"tok"))).unwrap();
        drive(fake.put(&password, &Secret::new(b"pw"))).unwrap();
        drive(fake.put(&db, &Secret::new(b"key"))).unwrap();
        assert_eq!(drive(fake.get(&token)).unwrap().unwrap().as_bytes(), b"tok");
        assert_eq!(
            drive(fake.get(&password)).unwrap().unwrap().as_bytes(),
            b"pw"
        );
        assert_eq!(drive(fake.get(&db)).unwrap().unwrap().as_bytes(), b"key");
        drive(fake.delete(&token)).unwrap();
        assert!(drive(fake.get(&token)).unwrap().is_none());
        drive(fake.delete(&token)).unwrap();
    }

    #[test]
    fn the_fake_implements_notifier_and_network() {
        let fake = Fake::new();
        assert_eq!(fake.path(), NetworkPath::Offline);
        fake.lock().path = NetworkPath::Metered;
        assert_eq!(fake.path(), NetworkPath::Metered);
        let notice = Notification {
            account: AccountId::new("a"),
            title: "New mail".into(),
            body: "preview".into(),
        };
        fake.notify(&notice);
        fake.set_badge(2);
        assert_eq!(fake.lock().notices, [notice]);
        assert_eq!(fake.lock().badge, 2);
        fake.set_badge(0);
        assert_eq!(fake.lock().badge, 0);
    }

    #[test]
    fn the_fake_implements_auth_session() {
        let fake = Fake::new();
        let secret = Secret::new("https://cb?code=one-time");
        fake.lock().auth = AuthResult::Completed(secret);
        let request = AuthRequest {
            account: AccountId::new("a"),
            authorization_url: "https://accounts.example/auth?challenge=abc".into(),
            callback: "mailune://oauth".into(),
        };
        let result = drive(fake.authenticate(&request)).unwrap();
        assert!(!format!("{result:?}").contains("one-time"));
        let AuthResult::Completed(secret) = result else {
            panic!("expected a completed session");
        };
        assert_eq!(secret.as_bytes(), b"https://cb?code=one-time");
        assert_eq!(
            fake.lock().seen_auth.as_ref().unwrap().callback,
            "mailune://oauth"
        );
    }

    #[test]
    fn the_fake_implements_platform_model() {
        let fake = Fake::new();
        let prompt = ModelPrompt {
            text: "secret-mail".into(),
            max_output_tokens: 8,
        };
        let err = drive(fake.complete(&prompt)).unwrap_err();
        assert!(!err.to_string().contains("secret-mail"));
        assert!(fake.capabilities().is_empty());

        fake.lock().models.push(ModelCapability {
            id: "foundation".into(),
            context_tokens: Some(4096),
        });
        fake.lock().reply = "done".into();
        assert_eq!(fake.capabilities()[0].id, "foundation");
        assert_eq!(drive(fake.complete(&prompt)).unwrap(), "done");
    }

    #[test]
    fn the_fake_implements_background_scheduler() {
        let fake = Fake::new();
        let account = AccountId::new("a");
        fake.schedule(&account, BackgroundWork::Sync).unwrap();
        fake.schedule(&account, BackgroundWork::Sync).unwrap();
        fake.schedule(&account, BackgroundWork::Bodies).unwrap();
        assert_eq!(
            fake.lock().jobs,
            [
                (account.clone(), BackgroundWork::Sync),
                (account.clone(), BackgroundWork::Bodies),
            ]
        );
        fake.cancel(&account, BackgroundWork::Sync).unwrap();
        fake.cancel(&account, BackgroundWork::Sync).unwrap();
        assert_eq!(fake.lock().jobs, [(account, BackgroundWork::Bodies)]);
    }
}
