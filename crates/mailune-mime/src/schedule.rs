//! Turn a sentence that names a date and a time into an iCalendar suggestion.
//!
//! The end is one hour after the start because the sentence only names a
//! start. The time is floating: the sentence has no zone. Nothing is sent.

use chrono::{Duration, NaiveDate};
use icalendar::{Calendar, Component, Event, EventLike};

/// A VEVENT the reader can offer. `ics` is the calendar text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSuggestion {
    /// The sentence, trimmed.
    pub summary: String,
    /// A VCALENDAR containing one VEVENT.
    pub ics: String,
}

/// Build an ICS suggestion when `sentence` contains both a date and a time.
pub fn suggest_event(sentence: &str) -> Option<EventSuggestion> {
    let summary = sentence.trim();
    if summary.is_empty() {
        return None;
    }
    let lower = summary.to_ascii_lowercase();
    let (date, span) = find_date(&lower)?;
    let (hour, minute) = find_time(&lower, span)?;
    let start =
        NaiveDate::from_ymd_opt(date.year, date.month, date.day)?.and_hms_opt(hour, minute, 0)?;
    let end = start.checked_add_signed(Duration::hours(1))?;
    let uid = format!(
        "mailune-{}{:02}{:02}T{:02}{:02}00@local",
        date.year, date.month, date.day, hour, minute
    );
    let event = Event::new()
        .summary(summary)
        .uid(&uid)
        .starts(start)
        .ends(end)
        .done();
    let ics = Calendar::new().push(event).done().to_string();
    Some(EventSuggestion {
        summary: summary.to_string(),
        ics,
    })
}

struct CivilDate {
    year: i32,
    month: u32,
    day: u32,
}

fn find_date(lower: &str) -> Option<(CivilDate, (usize, usize))> {
    find_iso(lower).or_else(|| find_month(lower))
}

fn find_iso(text: &str) -> Option<(CivilDate, (usize, usize))> {
    let bytes = text.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    for start in 0..=bytes.len() - 10 {
        if bytes[start + 4] != b'-' || bytes[start + 7] != b'-' {
            continue;
        }
        if !digits(&bytes[start..start + 4])
            || !digits(&bytes[start + 5..start + 7])
            || !digits(&bytes[start + 8..start + 10])
        {
            continue;
        }
        if !bare_edge(text, start, true) || !bare_edge(text, start + 10, false) {
            continue;
        }
        let date = CivilDate {
            year: text[start..start + 4].parse().ok()?,
            month: text[start + 5..start + 7].parse().ok()?,
            day: text[start + 8..start + 10].parse().ok()?,
        };
        return Some((date, (start, start + 10)));
    }
    None
}

fn find_month(text: &str) -> Option<(CivilDate, (usize, usize))> {
    for (index, name) in MONTHS.iter().enumerate() {
        let Some(at) = word_at(text, name) else {
            continue;
        };
        let month = u32::try_from(index).ok()? + 1;
        if let Some((day, year, end)) = day_year_after(text, at + name.len()) {
            return Some((CivilDate { year, month, day }, (at, end)));
        }
        if let Some((day, day_start)) = number_before(text, at)
            && let Some((year, year_end)) = year_after(text, at + name.len())
        {
            return Some((CivilDate { year, month, day }, (day_start, year_end)));
        }
    }
    None
}

fn day_year_after(text: &str, from: usize) -> Option<(u32, i32, usize)> {
    let rest = text[from..]
        .trim_start()
        .trim_start_matches(',')
        .trim_start();
    let rest_at = text.len() - rest.len();
    let (day, after_day) = take_number(rest)?;
    let day = u32::try_from(day).ok()?;
    if !(1..=31).contains(&day) {
        return None;
    }
    let after_day = after_day.trim_start().trim_start_matches(',').trim_start();
    let (year, _) = take_number(after_day)?;
    if !(1900..=2100).contains(&year) {
        return None;
    }
    let end = rest_at + (rest.len() - after_day.len()) + digit_width(year);
    Some((day, year, end))
}

fn year_after(text: &str, from: usize) -> Option<(i32, usize)> {
    let rest = text[from..].trim_start();
    let at = text.len() - rest.len();
    let (year, _) = take_number(rest)?;
    if !(1900..=2100).contains(&year) {
        return None;
    }
    Some((year, at + digit_width(year)))
}

fn number_before(text: &str, at: usize) -> Option<(u32, usize)> {
    let before = text[..at].trim_end();
    if before.len() == at {
        return None;
    }
    let start = number_start(text, before.len())?;
    let day: u32 = text[start..before.len()].parse().ok()?;
    if (1..=31).contains(&day) {
        Some((day, start))
    } else {
        None
    }
}

fn number_start(text: &str, end: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut start = end;
    while start > 0 && bytes[start - 1].is_ascii_digit() {
        start -= 1;
    }
    if start == end { None } else { Some(start) }
}

fn take_number(text: &str) -> Option<(i32, &str)> {
    let mut len = 0;
    while text.as_bytes().get(len).is_some_and(u8::is_ascii_digit) {
        len += 1;
    }
    if len == 0 {
        return None;
    }
    Some((text[..len].parse().ok()?, &text[len..]))
}

fn digit_width(value: i32) -> usize {
    value.unsigned_abs().to_string().len()
}

fn find_time(text: &str, date_span: (usize, usize)) -> Option<(u32, u32)> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() || in_span(index, date_span) {
            index += 1;
            continue;
        }
        let start = index;
        let Some((hour, next)) = read_hour(bytes, index) else {
            index += 1;
            continue;
        };
        let (minute, next, had_colon) = read_minute(bytes, next);
        let marker = meridiem(text, next);
        if marker.is_none() && !had_colon {
            index = start + 1;
            continue;
        }
        let Some(hour) = apply_meridiem(hour, marker) else {
            index = start + 1;
            continue;
        };
        if minute > 59 {
            index = start + 1;
            continue;
        }
        return Some((hour, minute));
    }
    None
}

fn read_hour(bytes: &[u8], mut index: usize) -> Option<(u32, usize)> {
    let mut hour = 0u32;
    let mut count = 0;
    while index < bytes.len() && bytes[index].is_ascii_digit() && count < 2 {
        hour = hour * 10 + u32::from(bytes[index] - b'0');
        index += 1;
        count += 1;
    }
    if count == 0 {
        None
    } else {
        Some((hour, index))
    }
}

fn read_minute(bytes: &[u8], index: usize) -> (u32, usize, bool) {
    if index >= bytes.len() || bytes[index] != b':' {
        return (0, index, false);
    }
    let next = index + 1;
    if next + 1 >= bytes.len() || !bytes[next].is_ascii_digit() || !bytes[next + 1].is_ascii_digit()
    {
        return (0, index, false);
    }
    let minute = u32::from(bytes[next] - b'0') * 10 + u32::from(bytes[next + 1] - b'0');
    (minute, next + 2, true)
}

fn meridiem(text: &str, index: usize) -> Option<bool> {
    let rest = text[index..].trim_start();
    let at = text.len() - rest.len();
    if rest.starts_with("am") && word_right(text, at + 2) {
        Some(false)
    } else if rest.starts_with("pm") && word_right(text, at + 2) {
        Some(true)
    } else {
        None
    }
}

fn apply_meridiem(hour: u32, afternoon: Option<bool>) -> Option<u32> {
    match afternoon {
        Some(false) if hour == 12 => Some(0),
        Some(false) if (1..=11).contains(&hour) => Some(hour),
        Some(true) if hour == 12 => Some(12),
        Some(true) if (1..=11).contains(&hour) => Some(hour + 12),
        None if hour <= 23 => Some(hour),
        _ => None,
    }
}

fn in_span(index: usize, span: (usize, usize)) -> bool {
    index >= span.0 && index < span.1
}

fn word_at(text: &str, word: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(found) = text[offset..].find(word) {
        let at = offset + found;
        if word_left(text, at) && word_right(text, at + word.len()) {
            return Some(at);
        }
        offset = at + 1;
    }
    None
}

fn word_left(text: &str, index: usize) -> bool {
    index == 0 || !text.as_bytes()[index - 1].is_ascii_alphabetic()
}

fn word_right(text: &str, index: usize) -> bool {
    index >= text.len() || !text.as_bytes()[index].is_ascii_alphabetic()
}

fn bare_edge(text: &str, index: usize, before: bool) -> bool {
    let byte = if before {
        if index == 0 {
            return true;
        }
        text.as_bytes()[index - 1]
    } else if index >= text.len() {
        return true;
    } else {
        text.as_bytes()[index]
    };
    !byte.is_ascii_alphanumeric()
}

fn digits(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii_digit)
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use icalendar::{Calendar, CalendarComponent, Component};

    use super::suggest_event;

    #[test]
    fn a_sentence_with_a_date_and_a_time_becomes_ics() {
        let sentence = "Let's meet on 8 October 2026 at 3:00 PM to review the draft.";
        let suggestion = suggest_event(sentence).unwrap();
        assert_eq!(suggestion.summary, sentence);
        let calendar = Calendar::from_str(&suggestion.ics).unwrap();
        let Some(CalendarComponent::Event(event)) = calendar
            .iter()
            .find(|component| matches!(component, CalendarComponent::Event(_)))
        else {
            panic!("missing event");
        };
        assert_eq!(event.property_value("DTSTART"), Some("20261008T150000"));
        assert_eq!(event.property_value("DTEND"), Some("20261008T160000"));
        assert_eq!(event.get_summary(), Some(sentence));
        let iso = suggest_event("Ship on 2026-10-08 at 09:30.").unwrap();
        let calendar = Calendar::from_str(&iso.ics).unwrap();
        let Some(CalendarComponent::Event(event)) = calendar.iter().next() else {
            panic!("missing event");
        };
        assert_eq!(event.property_value("DTSTART"), Some("20261008T093000"));
        assert!(suggest_event("See you in October.").is_none());
        assert!(suggest_event("Meet 2026-02-31 at 10:00.").is_none());
    }
}
