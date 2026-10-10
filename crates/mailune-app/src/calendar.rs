//! Calendar view: invitations found in local mail, shown as suggested
//! events by day.
//!
//! The source is the `text/calendar` parts the person already has. Nothing
//! is fetched: Graph calendars (P25) and the scheduling assistant (A22) are
//! separate tasks, and a JMAP Calendars source waits until that RFC is
//! published. ICS from mail is untrusted, so one unreadable part is counted
//! and skipped, never fatal.

use mailune_mime::{InviteKind, parse_invite};

/// One suggested event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEntry {
    /// Event UID; a re-sent invite with the same UID replaces the older one.
    pub uid: String,
    /// Title.
    pub title: String,
    /// Start as the invite wrote it: a date, or a date and time.
    pub start: String,
    /// End as the invite wrote it, when it has one.
    pub end: Option<String>,
    /// Who sent the invite.
    pub organizer: Option<String>,
}

/// The events that start on one day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarDay {
    /// `YYYY-MM-DD`, taken from the start as written. Converting it to the
    /// viewer's time zone is the shell's job, which knows that zone.
    pub date: String,
    /// Events, earliest start first.
    pub entries: Vec<CalendarEntry>,
}

/// Days with suggestions, earliest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CalendarView {
    /// Days that have at least one event.
    pub days: Vec<CalendarDay>,
    /// Calendar parts that could not be read or had no start date.
    pub skipped: usize,
}

/// Builds the view from calendar parts, oldest message first.
///
/// Only REQUESTs are suggestions: a REPLY answers an invite and is not an
/// event of its own. When two parts share a UID the later one wins, because
/// an organizer re-sends the whole invite to change it.
pub fn calendar_view<'a>(parts: impl IntoIterator<Item = &'a str>) -> CalendarView {
    let mut entries: Vec<CalendarEntry> = Vec::new();
    let mut skipped = 0;
    for part in parts {
        let Ok(invite) = parse_invite(part) else {
            skipped += 1;
            continue;
        };
        if invite.kind != InviteKind::Request {
            continue;
        }
        let Some(start) = invite.start.filter(|start| day_of(start).is_some()) else {
            skipped += 1;
            continue;
        };
        entries.retain(|entry| entry.uid != invite.uid);
        entries.push(CalendarEntry {
            uid: invite.uid,
            title: invite.summary,
            start,
            end: invite.end,
            organizer: invite.organizer,
        });
    }
    // An all-day `20261012` sorts before `20261012T…` on the same day, which
    // is where all-day events belong.
    entries.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.title.cmp(&b.title)));
    let mut days: Vec<CalendarDay> = Vec::new();
    for entry in entries {
        let date = day_of(&entry.start).unwrap_or_default();
        match days.last_mut() {
            Some(day) if day.date == date => day.entries.push(entry),
            _ => days.push(CalendarDay {
                date,
                entries: vec![entry],
            }),
        }
    }
    CalendarView { days, skipped }
}

/// `YYYY-MM-DD` from an RFC 5545 DATE or DATE-TIME value.
fn day_of(start: &str) -> Option<String> {
    let digits = start.get(..8)?;
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{}-{}-{}",
        &digits[..4],
        &digits[4..6],
        &digits[6..]
    ))
}

#[cfg(test)]
mod tests {
    use super::{CalendarView, calendar_view};

    const STANDUP: &str = include_str!("../fixtures/standup.ics");
    const STANDUP_MOVED: &str = include_str!("../fixtures/standup-moved.ics");
    const OFFSITE: &str = include_str!("../fixtures/offsite.ics");
    const REPLY: &str = include_str!("../fixtures/reply.ics");

    fn titles(view: &CalendarView) -> Vec<(&str, Vec<&str>)> {
        view.days
            .iter()
            .map(|day| {
                let titles = day
                    .entries
                    .iter()
                    .map(|entry| entry.title.as_str())
                    .collect();
                (day.date.as_str(), titles)
            })
            .collect()
    }

    #[test]
    fn local_invites_become_suggestions_by_day() {
        let view = calendar_view([STANDUP, OFFSITE, REPLY]);
        assert_eq!(titles(&view), [("2026-10-12", vec!["Offsite", "Standup"])]);
        let standup = &view.days[0].entries[1];
        assert_eq!(standup.uid, "standup@example.com");
        assert_eq!(standup.organizer.as_deref(), Some("ada@example.com"));
        assert_eq!(standup.end.as_deref(), Some("20261012T091500Z"));
        assert_eq!(view.skipped, 0);
    }

    #[test]
    fn a_resent_invite_replaces_the_older_one() {
        let view = calendar_view([STANDUP, OFFSITE, STANDUP_MOVED]);
        assert_eq!(
            titles(&view),
            [
                ("2026-10-12", vec!["Offsite"]),
                ("2026-10-13", vec!["Standup (moved)"]),
            ]
        );
    }

    #[test]
    fn unreadable_or_undated_parts_are_counted_not_fatal() {
        let undated = STANDUP.replace("DTSTART:20261012T090000Z\r\n", "");
        let view = calendar_view(["not a calendar", undated.as_str(), OFFSITE]);
        assert_eq!(titles(&view), [("2026-10-12", vec!["Offsite"])]);
        assert_eq!(view.skipped, 2);
        assert_eq!(calendar_view([]), CalendarView::default());
    }
}
