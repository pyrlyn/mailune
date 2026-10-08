//! In-memory [`FakeHost`] for the nine host traits.
//!
//! A poisoned mutex is recovered rather than panicked: the bytes are not a
//! lock invariant, and a failed test must not freeze the next one.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::{Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime};

use mailune_protocol::{
    AccountId, AuthRequest, AuthResult, AuthSession, BackgroundScheduler, BackgroundWork, Clock,
    ConnId, Error, Fs, ModelCapability, ModelPrompt, Net, NetworkPath, NetworkState, Notification,
    Notifier, PlatformModel, Secret, SecretId, SecretStore,
};

struct Pipe {
    pending: VecDeque<u8>,
    written: Vec<u8>,
}

struct State {
    now: SystemTime,
    files: HashMap<PathBuf, Vec<u8>>,
    secrets: HashMap<SecretId, Secret>,
    notices: Vec<Notification>,
    badge: u32,
    path: NetworkPath,
    auth: AuthResult,
    seen_auth: Option<AuthRequest>,
    models: Vec<ModelCapability>,
    reply: String,
    last_prompt: Option<String>,
    jobs: Vec<(AccountId, BackgroundWork)>,
    next_conn: u64,
    dials: Vec<(String, u16)>,
    conns: HashMap<u64, Pipe>,
}

/// In-memory host. It never touches the keychain, the disk, or the network.
///
/// `Debug` prints only the type name. The state holds keychain bytes and
/// prompt text, and formatting the fake must not copy them out.
pub struct FakeHost {
    state: Mutex<State>,
}

impl std::fmt::Debug for FakeHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FakeHost")
    }
}

impl Default for FakeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHost {
    /// Empty host. The clock starts at the unix epoch and the path is offline.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                now: SystemTime::UNIX_EPOCH,
                files: HashMap::new(),
                secrets: HashMap::new(),
                notices: Vec::new(),
                badge: 0,
                path: NetworkPath::Offline,
                auth: AuthResult::Cancelled,
                seen_auth: None,
                models: Vec::new(),
                reply: String::new(),
                last_prompt: None,
                jobs: Vec::new(),
                next_conn: 1,
                dials: Vec::new(),
                conns: HashMap::new(),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Path later [`NetworkState::path`] calls return.
    pub fn set_path(&self, path: NetworkPath) {
        self.lock().path = path;
    }

    /// Result the next [`AuthSession::authenticate`] returns.
    pub fn set_auth(&self, result: AuthResult) {
        self.lock().auth = result;
    }

    /// Text the next successful [`PlatformModel::complete`] returns.
    pub fn set_reply(&self, reply: impl Into<String>) {
        self.lock().reply = reply.into();
    }

    /// Adds an on-device model. Completion fails until one is installed.
    pub fn install_model(&self, capability: ModelCapability) {
        self.lock().models.push(capability);
    }

    /// Banners posted so far, in order.
    pub fn notices(&self) -> Vec<Notification> {
        self.lock().notices.clone()
    }

    /// Last badge value. `0` is the start, and also a cleared badge.
    pub fn badge(&self) -> u32 {
        self.lock().badge
    }

    /// Background registrations still held.
    pub fn jobs(&self) -> Vec<(AccountId, BackgroundWork)> {
        self.lock().jobs.clone()
    }

    /// `connect` calls, in order.
    pub fn dials(&self) -> Vec<(String, u16)> {
        self.lock().dials.clone()
    }

    /// Bytes accepted by [`Net::write`] for `id`, when that connection is open.
    pub fn written(&self, id: ConnId) -> Option<Vec<u8>> {
        self.lock()
            .conns
            .get(&id.as_raw())
            .map(|pipe| pipe.written.clone())
    }

    /// Authorization request the host was shown, if any.
    pub fn seen_auth(&self) -> Option<AuthRequest> {
        self.lock().seen_auth.clone()
    }

    /// Prompt text last given to [`PlatformModel::complete`].
    ///
    /// Tests check that a scenario actually called the model. Do not log it.
    pub fn last_prompt(&self) -> Option<String> {
        self.lock().last_prompt.clone()
    }
}

/// Polls `future` once.
///
/// The fakes finish without waiting, and this crate has no runtime. A future
/// that is still pending is reported instead of blocked on.
///
/// # Errors
///
/// [`Error::Host`] when `future` is not ready after one poll.
pub fn poll_now<T>(future: impl Future<Output = T>) -> Result<T, Error> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => Ok(value),
        Poll::Pending => Err(Error::host("testkit.poll", "future waited")),
    }
}

fn unknown(operation: &'static str) -> Error {
    Error::host(operation, "unknown connection")
}

impl Net for FakeHost {
    async fn connect(&self, host: &str, port: u16) -> Result<ConnId, Error> {
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
        Ok(ConnId::from_raw(id))
    }

    async fn read(&self, id: ConnId, buf: &mut [u8]) -> Result<usize, Error> {
        let mut state = self.lock();
        let pipe = state
            .conns
            .get_mut(&id.as_raw())
            .ok_or_else(|| unknown("net.read"))?;
        if buf.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
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

    async fn write(&self, id: ConnId, buf: &[u8]) -> Result<usize, Error> {
        let mut state = self.lock();
        let pipe = state
            .conns
            .get_mut(&id.as_raw())
            .ok_or_else(|| unknown("net.write"))?;
        pipe.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    async fn close(&self, id: ConnId) -> Result<(), Error> {
        let mut state = self.lock();
        if state.conns.remove(&id.as_raw()).is_none() {
            return Err(unknown("net.close"));
        }
        Ok(())
    }
}

impl Clock for FakeHost {
    fn now(&self) -> SystemTime {
        self.lock().now
    }

    async fn sleep(&self, duration: Duration) {
        let mut state = self.lock();
        state.now = state.now.checked_add(duration).unwrap_or(state.now);
    }
}

impl Fs for FakeHost {
    async fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
        self.lock()
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| Error::NotFound {
                path: path.display().to_string(),
            })
    }

    async fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), Error> {
        self.lock().files.insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    async fn remove(&self, path: &Path) -> Result<(), Error> {
        self.lock().files.remove(path);
        Ok(())
    }
}

impl SecretStore for FakeHost {
    async fn get(&self, id: &SecretId) -> Result<Option<Secret>, Error> {
        Ok(self.lock().secrets.get(id).cloned())
    }

    async fn put(&self, id: &SecretId, secret: &Secret) -> Result<(), Error> {
        self.lock().secrets.insert(id.clone(), secret.clone());
        Ok(())
    }

    async fn delete(&self, id: &SecretId) -> Result<(), Error> {
        self.lock().secrets.remove(id);
        Ok(())
    }
}

impl Notifier for FakeHost {
    fn notify(&self, notice: &Notification) {
        self.lock().notices.push(notice.clone());
    }

    fn set_badge(&self, count: u32) {
        self.lock().badge = count;
    }
}

impl NetworkState for FakeHost {
    fn path(&self) -> NetworkPath {
        self.lock().path
    }
}

impl AuthSession for FakeHost {
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, Error> {
        let mut state = self.lock();
        state.seen_auth = Some(request.clone());
        Ok(state.auth.clone())
    }
}

impl PlatformModel for FakeHost {
    fn capabilities(&self) -> Vec<ModelCapability> {
        self.lock().models.clone()
    }

    async fn complete(&self, prompt: &ModelPrompt) -> Result<String, Error> {
        let mut state = self.lock();
        state.last_prompt = Some(prompt.text.clone());
        if state.models.is_empty() {
            return Err(Error::host("platform.complete", "no platform model"));
        }
        Ok(state.reply.clone())
    }
}

impl BackgroundScheduler for FakeHost {
    fn schedule(&self, account: &AccountId, work: BackgroundWork) -> Result<(), Error> {
        let mut state = self.lock();
        state
            .jobs
            .retain(|(id, kind)| !(id == account && *kind == work));
        state.jobs.push((account.clone(), work));
        Ok(())
    }

    fn cancel(&self, account: &AccountId, work: BackgroundWork) -> Result<(), Error> {
        self.lock()
            .jobs
            .retain(|(id, kind)| !(id == account && *kind == work));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use mailune_protocol::{
        AccountId, AuthRequest, AuthResult, AuthSession, BackgroundScheduler, BackgroundWork,
        Clock, Fs, ModelCapability, ModelPrompt, Net, NetworkPath, NetworkState, Notification,
        Notifier, PlatformModel, Secret, SecretId, SecretStore,
    };

    use super::{FakeHost, poll_now};

    #[test]
    fn net_reads_the_greeting_and_records_writes() {
        let host = FakeHost::new();
        let id = poll_now(host.connect("imap.example", 993))
            .unwrap()
            .unwrap();
        let mut buf = [0u8; 8];
        let count = poll_now(Net::read(&host, id, &mut buf)).unwrap().unwrap();
        assert_eq!(&buf[..count], b"READY");
        assert_eq!(
            poll_now(Net::write(&host, id, b"a001 NOOP"))
                .unwrap()
                .unwrap(),
            b"a001 NOOP".len()
        );
        assert_eq!(host.written(id).unwrap(), b"a001 NOOP");
        assert_eq!(host.dials(), [("imap.example".to_string(), 993)]);
        poll_now(host.close(id)).unwrap().unwrap();
        assert!(poll_now(host.close(id)).unwrap().is_err());
    }

    #[test]
    fn clock_sleep_advances_without_blocking() {
        let host = FakeHost::new();
        let start = host.now();
        poll_now(host.sleep(Duration::from_secs(5))).unwrap();
        assert_eq!(
            host.now().duration_since(start).unwrap(),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn files_round_trip_and_a_missing_path_is_not_found() {
        let host = FakeHost::new();
        let path = PathBuf::from("mail/draft");
        poll_now(Fs::write(&host, &path, b"hello"))
            .unwrap()
            .unwrap();
        assert_eq!(poll_now(Fs::read(&host, &path)).unwrap().unwrap(), b"hello");
        poll_now(host.remove(&path)).unwrap().unwrap();
        assert!(matches!(
            poll_now(Fs::read(&host, &path)).unwrap(),
            Err(mailune_protocol::Error::NotFound { .. })
        ));
        poll_now(host.remove(&path)).unwrap().unwrap();
    }

    #[test]
    fn secrets_round_trip_without_a_keychain() {
        let host = FakeHost::new();
        let id = SecretId::token(AccountId::new("a"));
        assert!(
            poll_now(SecretStore::get(&host, &id))
                .unwrap()
                .unwrap()
                .is_none()
        );
        poll_now(host.put(&id, &Secret::new(b"tok")))
            .unwrap()
            .unwrap();
        assert_eq!(
            poll_now(host.get(&id))
                .unwrap()
                .unwrap()
                .unwrap()
                .as_bytes(),
            b"tok"
        );
        poll_now(host.delete(&id)).unwrap().unwrap();
        assert!(poll_now(host.get(&id)).unwrap().unwrap().is_none());
        poll_now(host.delete(&id)).unwrap().unwrap();
    }

    #[test]
    fn notifier_and_network_path_are_readable() {
        let host = FakeHost::new();
        assert_eq!(host.path(), NetworkPath::Offline);
        host.set_path(NetworkPath::Unmetered);
        assert_eq!(host.path(), NetworkPath::Unmetered);
        let notice = Notification {
            account: AccountId::new("a"),
            title: "New mail".into(),
            body: "preview".into(),
        };
        host.notify(&notice);
        host.set_badge(2);
        assert_eq!(host.notices(), [notice]);
        assert_eq!(host.badge(), 2);
    }

    #[test]
    fn auth_returns_the_scripted_result() {
        let host = FakeHost::new();
        host.set_auth(AuthResult::Completed(Secret::new(b"code")));
        let request = AuthRequest {
            account: AccountId::new("a"),
            authorization_url: "https://accounts.example/auth".into(),
            callback: "mailune://oauth".into(),
        };
        let result = poll_now(host.authenticate(&request)).unwrap().unwrap();
        assert!(matches!(result, AuthResult::Completed(_)));
        assert_eq!(host.seen_auth().unwrap().callback, "mailune://oauth");
    }

    #[test]
    fn platform_model_hides_the_prompt_when_nothing_is_installed() {
        let host = FakeHost::new();
        let prompt = ModelPrompt {
            text: "secret-mail".into(),
            max_output_tokens: 8,
        };
        let err = poll_now(host.complete(&prompt)).unwrap().unwrap_err();
        assert!(!err.to_string().contains("secret-mail"));
        assert!(host.capabilities().is_empty());
        host.install_model(ModelCapability {
            id: "foundation".into(),
            context_tokens: Some(128),
        });
        host.set_reply("done");
        assert_eq!(poll_now(host.complete(&prompt)).unwrap().unwrap(), "done");
        assert_eq!(host.last_prompt().as_deref(), Some("secret-mail"));
    }

    #[test]
    fn scheduler_replaces_the_same_kind_and_cancel_is_idempotent() {
        let host = FakeHost::new();
        let account = AccountId::new("a");
        host.schedule(&account, BackgroundWork::Sync).unwrap();
        host.schedule(&account, BackgroundWork::Sync).unwrap();
        host.schedule(&account, BackgroundWork::Bodies).unwrap();
        assert_eq!(
            host.jobs(),
            [
                (account.clone(), BackgroundWork::Sync),
                (account.clone(), BackgroundWork::Bodies),
            ]
        );
        host.cancel(&account, BackgroundWork::Sync).unwrap();
        host.cancel(&account, BackgroundWork::Sync).unwrap();
        assert_eq!(host.jobs(), [(account, BackgroundWork::Bodies)]);
    }
}
