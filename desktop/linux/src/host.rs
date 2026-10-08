//! In-memory stand-in for the Linux host.
//!
//! Secrets, banners, the network path, and the OAuth redirect stay in this
//! process. Nothing here opens a socket or reads a credential from the OS.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, MutexGuard};

use mailune_protocol::{
    AuthRequest, AuthResult, AuthSession, Error, NetworkPath, NetworkState, Notification, Notifier,
    Secret, SecretId, SecretStore,
};

struct State {
    secrets: HashMap<SecretId, Secret>,
    notices: Vec<Notification>,
    badge: u32,
    path: NetworkPath,
    redirect: Option<String>,
    callback: Option<String>,
}

/// Host whose four surfaces are fakes.
///
/// `Debug` prints only the type name, so a secret stored here cannot leak
/// through formatting.
pub struct LinuxHost {
    state: Mutex<State>,
}

impl std::fmt::Debug for LinuxHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LinuxHost")
    }
}

impl LinuxHost {
    /// Empty host. The path is offline and no redirect is scripted.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                secrets: HashMap::new(),
                notices: Vec::new(),
                badge: 0,
                path: NetworkPath::Offline,
                redirect: None,
                callback: None,
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

    /// Redirect [`AuthSession::authenticate`] completes with.
    ///
    /// `url` is the callback the fake returns. It is not requested.
    pub fn set_redirect(&self, url: impl Into<String>) {
        self.lock().redirect = Some(url.into());
    }

    /// Banners posted so far, in order.
    pub fn notices(&self) -> Vec<Notification> {
        self.lock().notices.clone()
    }

    /// Last badge value. `0` is both the start and a cleared badge.
    pub fn badge(&self) -> u32 {
        self.lock().badge
    }

    /// Callback the last authorization request asked the host to catch.
    pub fn callback(&self) -> Option<String> {
        self.lock().callback.clone()
    }
}

impl Default for LinuxHost {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for LinuxHost {
    fn get(&self, id: &SecretId) -> impl Future<Output = Result<Option<Secret>, Error>> + Send {
        let found = self.lock().secrets.get(id).cloned();
        std::future::ready(Ok(found))
    }

    fn put(
        &self,
        id: &SecretId,
        secret: &Secret,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.lock().secrets.insert(id.clone(), secret.clone());
        std::future::ready(Ok(()))
    }

    fn delete(&self, id: &SecretId) -> impl Future<Output = Result<(), Error>> + Send {
        self.lock().secrets.remove(id);
        std::future::ready(Ok(()))
    }
}

impl Notifier for LinuxHost {
    fn notify(&self, notice: &Notification) {
        self.lock().notices.push(notice.clone());
    }

    fn set_badge(&self, count: u32) {
        self.lock().badge = count;
    }
}

impl NetworkState for LinuxHost {
    fn path(&self) -> NetworkPath {
        self.lock().path
    }
}

impl AuthSession for LinuxHost {
    fn authenticate(
        &self,
        request: &AuthRequest,
    ) -> impl Future<Output = Result<AuthResult, Error>> + Send {
        let mut state = self.lock();
        state.callback = Some(request.callback.clone());
        let result = match state.redirect.clone() {
            Some(url) => AuthResult::Completed(Secret::new(url.into_bytes())),
            None => AuthResult::Cancelled,
        };
        drop(state);
        std::future::ready(Ok(result))
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use mailune_protocol::{
        AccountId, AuthRequest, AuthResult, AuthSession, NetworkPath, NetworkState, Notification,
        Notifier, Secret, SecretId, SecretStore,
    };

    use super::LinuxHost;

    fn wait<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the fake host is ready immediately"),
        }
    }

    #[test]
    fn a_secret_round_trips_and_debug_hides_it() {
        let host = LinuxHost::new();
        let id = SecretId::token(AccountId::new("acc"));
        let secret = Secret::new(b"token-value");
        wait(host.put(&id, &secret)).unwrap();
        let loaded = wait(host.get(&id)).unwrap().unwrap();
        assert_eq!(loaded.as_bytes(), b"token-value");
        assert!(!format!("{host:?}").contains("token-value"));
        wait(host.delete(&id)).unwrap();
        assert!(wait(host.get(&id)).unwrap().is_none());
    }

    #[test]
    fn a_notification_and_badge_are_recorded() {
        let host = LinuxHost::new();
        host.notify(&Notification {
            account: AccountId::new("acc"),
            title: "Mail".into(),
            body: "One new message".into(),
        });
        host.set_badge(2);
        assert_eq!(host.notices()[0].title, "Mail");
        assert_eq!(host.badge(), 2);
    }

    #[test]
    fn the_network_path_is_the_scripted_one() {
        let host = LinuxHost::new();
        assert_eq!(host.path(), NetworkPath::Offline);
        host.set_path(NetworkPath::Unmetered);
        assert_eq!(host.path(), NetworkPath::Unmetered);
    }

    #[test]
    fn an_oauth_redirect_is_the_scripted_callback() {
        let host = LinuxHost::new();
        let redirect = "mailune://callback?code=abc";
        host.set_redirect(redirect);
        let request = AuthRequest {
            account: AccountId::new("acc"),
            authorization_url: "mailune://authorize".into(),
            callback: "mailune://callback".into(),
        };
        let result = wait(host.authenticate(&request)).unwrap();
        match result {
            AuthResult::Completed(secret) => assert_eq!(secret.as_bytes(), redirect.as_bytes()),
            AuthResult::Cancelled => panic!("the redirect was scripted"),
        }
        assert_eq!(host.callback().as_deref(), Some("mailune://callback"));
    }
}
