//! App lock: an idle timer and a biometric request the host would fulfill.
//!
//! Nothing here talks to a biometric sensor. [`BiometricRequest`] is a value
//! the host accepts or rejects. A locked app refuses send until that request
//! is fulfilled.

use std::time::{Duration, SystemTime};

use crate::Error;

/// What the host shows when the person must unlock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiometricRequest {
    /// Matches this request to the one the lock is waiting on.
    pub challenge: u64,
    /// Sentence the host may show. Not a secret.
    pub reason: String,
}

/// Idle timer plus the pending unlock request.
pub struct AppLock {
    idle_limit: Duration,
    last_activity: SystemTime,
    locked: bool,
    pending: Option<BiometricRequest>,
    next_challenge: u64,
}

impl AppLock {
    /// Starts unlocked. `idle_limit` is how long after the last activity the
    /// app locks. `now` is that last activity.
    pub fn new(idle_limit: Duration, now: SystemTime) -> Self {
        Self {
            idle_limit,
            last_activity: now,
            locked: false,
            pending: None,
            next_challenge: 1,
        }
    }

    /// Records that the person used the app at `now`.
    pub fn note_activity(&mut self, now: SystemTime) {
        self.last_activity = now;
    }

    /// Locks when `now` is at least `idle_limit` after the last activity.
    ///
    /// A clock step backwards is not idle: the person was active later than
    /// `now`, so the app stays as it was.
    pub fn poll(&mut self, now: SystemTime) {
        let idle = now
            .duration_since(self.last_activity)
            .unwrap_or(Duration::ZERO);
        if idle >= self.idle_limit {
            self.locked = true;
        }
    }

    /// Whether send must wait for an unlock.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// The request the host should fulfill. A newer call replaces the previous one.
    pub fn request_unlock(&mut self) -> BiometricRequest {
        let request = BiometricRequest {
            challenge: self.next_challenge,
            reason: "Unlock to send mail".into(),
        };
        self.next_challenge = self.next_challenge.saturating_add(1);
        self.pending = Some(request.clone());
        request
    }

    /// Applies the host's answer. A challenge that is not the pending one is ignored.
    pub fn fulfill(&mut self, request: &BiometricRequest, accepted: bool, now: SystemTime) {
        let matches = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.challenge == request.challenge);
        if !matches {
            return;
        }
        self.pending = None;
        if accepted {
            self.locked = false;
            self.last_activity = now;
        }
    }

    /// Allows send only while the app is unlocked.
    ///
    /// # Errors
    ///
    /// [`Error::Locked`] when the idle timer has locked the app and the host
    /// has not accepted the current biometric request.
    pub fn authorize_send(&self) -> Result<(), Error> {
        if self.locked {
            Err(Error::Locked)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::AppLock;
    use crate::Error;

    #[test]
    fn a_locked_app_rejects_send_until_unlock() {
        let start = SystemTime::UNIX_EPOCH;
        let mut lock = AppLock::new(Duration::from_secs(30), start);
        assert!(lock.authorize_send().is_ok());
        lock.poll(start + Duration::from_secs(29));
        assert!(!lock.is_locked());
        lock.poll(start + Duration::from_secs(30));
        assert!(lock.is_locked());
        assert!(matches!(lock.authorize_send(), Err(Error::Locked)));

        let rejected = lock.request_unlock();
        let now = start + Duration::from_secs(31);
        lock.fulfill(&rejected, false, now);
        assert!(matches!(lock.authorize_send(), Err(Error::Locked)));

        let stale = lock.request_unlock();
        let current = lock.request_unlock();
        lock.fulfill(&stale, true, now);
        assert!(matches!(lock.authorize_send(), Err(Error::Locked)));
        lock.fulfill(&current, true, now);
        assert!(lock.authorize_send().is_ok());
    }
}
