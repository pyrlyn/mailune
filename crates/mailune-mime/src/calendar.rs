//! iCalendar REQUEST and REPLY.
//!
//! `icalendar` 0.17.14 (crates.io, checked 2026-10-08) is maintained and
//! already parses RFC 5545, so this module only keeps the fields an invite
//! card shows. Nothing is sent.

use std::str::FromStr;

use icalendar::{Attendee, Calendar, CalendarComponent, Component, PartStat};

use crate::Error;

/// Which invitation this calendar is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InviteKind {
    /// An organizer asked for a response.
    Request,
    /// An attendee answered.
    Reply,
}

/// The invite fields a reader shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invite {
    /// REQUEST or REPLY.
    pub kind: InviteKind,
    /// Stable event id.
    pub uid: String,
    /// Title.
    pub summary: String,
    /// Start, as the calendar wrote it.
    pub start: Option<String>,
    /// End, as the calendar wrote it.
    pub end: Option<String>,
    /// Organizer address, without a `mailto:` prefix.
    pub organizer: Option<String>,
    /// First attendee address, without a `mailto:` prefix.
    pub attendee: Option<String>,
    /// That attendee's PARTSTAT, when the calendar set one.
    pub partstat: Option<String>,
}

/// Read one REQUEST or REPLY. Other methods, and calendars with no event, fail.
pub fn parse_invite(ics: &str) -> Result<Invite, Error> {
    let calendar = Calendar::from_str(ics).map_err(|_| Error::Calendar)?;
    let kind = method(&calendar).ok_or(Error::Calendar)?;
    let Some(CalendarComponent::Event(event)) = calendar
        .iter()
        .find(|component| matches!(component, CalendarComponent::Event(_)))
    else {
        return Err(Error::Calendar);
    };
    let uid = event.get_uid().ok_or(Error::Calendar)?.to_owned();
    let guest = event.get_attendees().into_iter().next();
    Ok(Invite {
        kind,
        uid,
        summary: event.get_summary().unwrap_or("").to_owned(),
        start: event.property_value("DTSTART").map(str::to_owned),
        end: event.property_value("DTEND").map(str::to_owned),
        organizer: event.property_value("ORGANIZER").map(address),
        attendee: guest.as_ref().map(|person| address(&person.cal_address)),
        partstat: guest.as_ref().and_then(partstat),
    })
}

fn method(calendar: &Calendar) -> Option<InviteKind> {
    calendar.properties.iter().find_map(|property| {
        if !property.key().eq_ignore_ascii_case("METHOD") {
            return None;
        }
        match property.value().trim().to_ascii_uppercase().as_str() {
            "REQUEST" => Some(InviteKind::Request),
            "REPLY" => Some(InviteKind::Reply),
            _ => None,
        }
    })
}

fn partstat(person: &Attendee) -> Option<String> {
    person.part_stat.map(|value| {
        match value {
            PartStat::NeedsAction => "NEEDS-ACTION",
            PartStat::Accepted => "ACCEPTED",
            PartStat::Declined => "DECLINED",
            PartStat::Tentative => "TENTATIVE",
            PartStat::Delegated => "DELEGATED",
            PartStat::Completed => "COMPLETED",
            PartStat::InProcess => "IN-PROCESS",
        }
        .to_owned()
    })
}

fn address(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix("mailto:")
        .or_else(|| trimmed.strip_prefix("MAILTO:"))
        .unwrap_or(trimmed)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_reply_become_invites() {
        let request = "\
BEGIN:VCALENDAR\r\n\
METHOD:REQUEST\r\n\
PRODID:-//Mailune//EN\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:invite-1@example.com\r\n\
SUMMARY:Planning\r\n\
DTSTART:20261008T150000Z\r\n\
DTEND:20261008T160000Z\r\n\
ORGANIZER:mailto:ana@example.com\r\n\
ATTENDEE;PARTSTAT=NEEDS-ACTION:mailto:bo@example.com\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";
        let invite = parse_invite(request).unwrap();
        assert_eq!(invite.kind, InviteKind::Request);
        assert_eq!(invite.uid, "invite-1@example.com");
        assert_eq!(invite.summary, "Planning");
        assert_eq!(invite.start.as_deref(), Some("20261008T150000Z"));
        assert_eq!(invite.organizer.as_deref(), Some("ana@example.com"));
        assert_eq!(invite.attendee.as_deref(), Some("bo@example.com"));
        assert_eq!(invite.partstat.as_deref(), Some("NEEDS-ACTION"));

        let reply = "\
BEGIN:VCALENDAR\r\n\
METHOD:REPLY\r\n\
PRODID:-//Mailune//EN\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:invite-1@example.com\r\n\
SUMMARY:Planning\r\n\
ATTENDEE;PARTSTAT=ACCEPTED:mailto:bo@example.com\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";
        let invite = parse_invite(reply).unwrap();
        assert_eq!(invite.kind, InviteKind::Reply);
        assert_eq!(invite.partstat.as_deref(), Some("ACCEPTED"));
        assert!(parse_invite("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n").is_err());
    }
}
