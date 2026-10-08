//! Whether a new message should notify the person.
//!
//! VIP, quiet hours, and a priority number are the whole decision. The
//! priority is a plain `u8` so this crate does not depend on the model
//! crate. Nothing here posts a system banner.

/// A window of the day, in minutes from local midnight.
///
/// `end_minute` is exclusive. When `start_minute` is greater than
/// `end_minute`, the window crosses midnight. Equal ends are an empty window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuietHours {
    /// First quiet minute, `0..1440`.
    pub start_minute: u16,
    /// First minute that is not quiet, `0..1440`.
    pub end_minute: u16,
}

/// Notify or stay quiet. The host turns [`Decision::Notify`] into a banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The person should see this message.
    Notify,
    /// The person should not be interrupted.
    Suppress,
}

/// VIP addresses, quiet hours, and the lowest priority that still notifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticePolicy {
    vip: Vec<String>,
    quiet: QuietHours,
    min_priority: u8,
}

impl NoticePolicy {
    /// Builds a policy. VIP addresses are matched case-insensitively.
    ///
    /// Blank addresses are dropped so an empty entry cannot match every sender.
    pub fn new(
        vip: impl IntoIterator<Item = impl Into<String>>,
        quiet: QuietHours,
        min_priority: u8,
    ) -> Self {
        let mut vip: Vec<String> = vip
            .into_iter()
            .map(|address| address.into().trim().to_ascii_lowercase())
            .filter(|address| !address.is_empty())
            .collect();
        vip.sort();
        vip.dedup();
        Self {
            vip,
            quiet,
            min_priority,
        }
    }

    /// Decides for `from` at `minute_of_day` with `priority`.
    ///
    /// A VIP notifies even during quiet hours: the person listed that address
    /// so a night message from them is the exception. Everyone else is
    /// suppressed for the whole quiet window, whatever the score.
    pub fn decide(&self, from: &str, priority: u8, minute_of_day: u16) -> Decision {
        if self.is_vip(from) {
            return Decision::Notify;
        }
        if in_quiet(self.quiet, minute_of_day) || priority < self.min_priority {
            Decision::Suppress
        } else {
            Decision::Notify
        }
    }

    fn is_vip(&self, from: &str) -> bool {
        let address = sender_address(from).to_ascii_lowercase();
        self.vip.iter().any(|vip| vip == &address)
    }
}

fn sender_address(from: &str) -> &str {
    from.rsplit(['<', ' '])
        .next()
        .unwrap_or(from)
        .trim()
        .trim_end_matches('>')
}

fn in_quiet(quiet: QuietHours, minute: u16) -> bool {
    // A value outside a day, or a zero-length window, is not quiet. Guessing
    // a whole day of silence from a bad clock would hide mail.
    if quiet.start_minute >= 1440
        || quiet.end_minute >= 1440
        || quiet.start_minute == quiet.end_minute
        || minute >= 1440
    {
        return false;
    }
    if quiet.start_minute < quiet.end_minute {
        minute >= quiet.start_minute && minute < quiet.end_minute
    } else {
        minute >= quiet.start_minute || minute < quiet.end_minute
    }
}

#[cfg(test)]
mod tests {
    use super::{Decision, NoticePolicy, QuietHours};

    fn policy() -> NoticePolicy {
        NoticePolicy::new(
            ["Ada@Acme.io", "ada@acme.io", ""],
            QuietHours {
                start_minute: 22 * 60,
                end_minute: 7 * 60,
            },
            40,
        )
    }

    #[test]
    fn vip_quiet_hours_and_priority_choose_notify_or_suppress() {
        let policy = policy();
        assert_eq!(
            policy.decide("Ada Lovelace <ada@acme.io>", 1, 23 * 60),
            Decision::Notify
        );
        assert_eq!(
            policy.decide("Bea <bea@acme.io>", 90, 23 * 60),
            Decision::Suppress
        );
        assert_eq!(
            policy.decide("Bea <bea@acme.io>", 90, 12 * 60),
            Decision::Notify
        );
        assert_eq!(
            policy.decide("Bea <bea@acme.io>", 10, 12 * 60),
            Decision::Suppress
        );
        assert_eq!(
            policy.decide("Bea <bea@acme.io>", 40, 6 * 60),
            Decision::Suppress
        );
        assert_eq!(
            policy.decide("Bea <bea@acme.io>", 40, 7 * 60),
            Decision::Notify
        );
    }
}
