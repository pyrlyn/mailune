//! Cooperative cancellation and a progress counter.
//!
//! The token is polled, not signalled. A long loop checks it between units,
//! so cancel is observed on the next iteration instead of waiting out the work.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::Error;

/// Shared cancel flag. Clones see the same flag, so the host keeps one copy
/// and the operation keeps another.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    /// A token that has not been cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks every clone to stop at its next [`CancelToken::check`].
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Whether [`CancelToken::cancel`] has been called.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// `Err` once cancelled, so a checkpoint can use `?`.
    ///
    /// # Errors
    ///
    /// [`Error::Cancelled`] when the token has been cancelled.
    pub fn check(&self) -> Result<(), Error> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// How far a cancellable loop has got.
///
/// Shared so a caller can read it while the loop runs. Relaxed ordering is
/// enough: the number is a hint, not a lock.
#[derive(Debug, Clone)]
pub struct Progress {
    done: Arc<AtomicU64>,
    total: u64,
}

impl Progress {
    /// A counter that starts at zero. `total` is the planned number of units.
    pub fn new(total: u64) -> Self {
        Self {
            done: Arc::new(AtomicU64::new(0)),
            total,
        }
    }

    /// Records one finished unit. Stops at [`u64::MAX`] instead of wrapping.
    pub fn tick(&self) {
        let _ = self
            .done
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |done| {
                done.checked_add(1)
            });
    }

    /// Units finished so far.
    pub fn completed(&self) -> u64 {
        self.done.load(Ordering::Relaxed)
    }

    /// Units the loop was asked to finish.
    pub fn total(&self) -> u64 {
        self.total
    }
}

/// Runs until `progress` is finished, or `token` is cancelled.
///
/// The check is the first step of each unit, so a cancel is seen before the
/// next one. The function does not sleep.
///
/// # Errors
///
/// [`Error::Cancelled`] when the token is cancelled before the total is reached.
pub fn run_loop(token: &CancelToken, progress: &Progress) -> Result<(), Error> {
    while progress.completed() < progress.total() {
        token.check()?;
        progress.tick();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{CancelToken, Progress, run_loop};
    use crate::Error;

    #[test]
    fn a_clone_sees_cancel() {
        let token = CancelToken::new();
        let other = token.clone();
        assert!(other.check().is_ok());
        token.cancel();
        assert!(other.is_cancelled());
        assert!(matches!(other.check(), Err(Error::Cancelled)));
    }

    #[test]
    fn a_long_loop_stops_within_100ms_of_cancel() {
        let token = CancelToken::new();
        let progress = Progress::new(u64::MAX);
        let worker_token = token.clone();
        let worker_progress = progress.clone();
        let started = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&started);
        let handle = thread::spawn(move || {
            flag.store(true, Ordering::SeqCst);
            run_loop(&worker_token, &worker_progress)
        });
        while !started.load(Ordering::SeqCst) || progress.completed() == 0 {
            thread::yield_now();
        }
        let cancelled_at = Instant::now();
        token.cancel();
        let result = handle.join().expect("worker");
        assert!(cancelled_at.elapsed() < Duration::from_millis(100));
        assert!(matches!(result, Err(Error::Cancelled)));
        assert!(progress.completed() > 0);
        assert!(progress.completed() < progress.total());
    }
}
