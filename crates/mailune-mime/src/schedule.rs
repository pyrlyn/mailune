//! Scheduling extraction: a sentence with a date and a time becomes an ICS
//! suggestion.
//!
//! Heuristics, no model. The time has no zone in prose, so the event uses
//! RFC 5545 floating local time and the calendar app places it in the
//! person's zone. Date arithmetic is the civil-calendar algorithm below
//! because the tree has no date crate this crate may use (`icalendar` only
//! builds text here). Nothing is sent and no calendar server is called.

use icalendar::{Calendar, Component, Event};

/// A calendar day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDate {
    /// Year.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day, 1 to 31.
    pub day: u8,
}

/// A suggested event, ready to offer as an `.ics` attachment or card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleSuggestion {
    /// Day of the event.
    pub date: CivilDate,
    /// Start, hour 0 to 23.
    pub hour: u8,
    /// Start minute.
    pub minute: u8,
    /// Floating local start, `YYYYMMDDTHHMMSS`.
    pub start: String,
    /// Floating local end, one hour later.
    pub end: String,
    /// The calendar text.
    pub ics: String,
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
const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// Finds a date and a time in `text`, relative to `today`, and builds a
/// one-hour event titled `summary` with id `uid`. `None` unless both are found.
pub fn suggest_event(
    text: &str,
    today: CivilDate,
    summary: &str,
    uid: &str,
) -> Option<ScheduleSuggestion> {
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !(c.is_alphanumeric() || c == ':' || c == '-'))
        .filter(|word| !word.is_empty())
        .collect();
    let date = find_date(&words, today)?;
    let (hour, minute) = find_time(&words)?;
    let start = stamp(date, hour, minute);
    let end_hour = hour + 1;
    let end = if end_hour == 24 {
        stamp(add_days(date, 1), 0, minute)
    } else {
        stamp(date, end_hour, minute)
    };
    let event = Event::new()
        .uid(uid)
        .summary(summary)
        .add_property("DTSTART", &start)
        .add_property("DTEND", &end)
        .done();
    let ics = Calendar::new().push(event).done().to_string();
    Some(ScheduleSuggestion {
        date,
        hour,
        minute,
        start,
        end,
        ics,
    })
}

fn stamp(date: CivilDate, hour: u8, minute: u8) -> String {
    format!(
        "{:04}{:02}{:02}T{hour:02}{minute:02}00",
        date.year, date.month, date.day
    )
}

fn find_date(words: &[&str], today: CivilDate) -> Option<CivilDate> {
    for (index, word) in words.iter().enumerate() {
        if let Some(date) = iso_date(word) {
            return Some(date);
        }
        match *word {
            "today" => return Some(today),
            "tomorrow" => return Some(add_days(today, 1)),
            _ => {}
        }
        if let Some(weekday) = WEEKDAYS.iter().position(|day| day == word) {
            let now = weekday_of(today);
            let ahead = (weekday + 7 - now) % 7;
            // "On Friday" said on a Friday means the next one.
            let ahead = if ahead == 0 { 7 } else { ahead };
            return Some(add_days(today, i64::try_from(ahead).ok()?));
        }
        if let Some(month) = month_of(word) {
            let day = words
                .get(index + 1)
                .and_then(|next| day_of(next))
                .or_else(|| index.checked_sub(1).and_then(|i| day_of(words[i])));
            if let Some(day) = day {
                return next_occurrence(month, day, today);
            }
        }
    }
    None
}

fn find_time(words: &[&str]) -> Option<(u8, u8)> {
    for (index, word) in words.iter().enumerate() {
        if *word == "noon" {
            return Some((12, 0));
        }
        for (suffix, offset) in [("am", 0), ("pm", 12)] {
            let clock = word.strip_suffix(suffix).filter(|rest| !rest.is_empty());
            let split = index
                .checked_sub(1)
                .filter(|_| *word == suffix)
                .map(|i| words[i]);
            if let Some((hour, minute)) = clock.or(split).and_then(clock_parts)
                && (1..=12).contains(&hour)
            {
                return Some((hour % 12 + offset, minute));
            }
        }
        if word.contains(':')
            && let Some((hour, minute)) = clock_parts(word)
            && hour < 24
        {
            return Some((hour, minute));
        }
    }
    None
}

fn clock_parts(text: &str) -> Option<(u8, u8)> {
    let (hour, minute) = text.split_once(':').unwrap_or((text, "0"));
    let hour: u8 = hour.parse().ok()?;
    let minute: u8 = minute.parse().ok()?;
    (minute < 60).then_some((hour, minute))
}

fn iso_date(word: &str) -> Option<CivilDate> {
    let mut parts = word.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 {
        return None;
    }
    let date = CivilDate {
        year: year.parse().ok()?,
        month: month.parse().ok()?,
        day: day.parse().ok()?,
    };
    valid(date).then_some(date)
}

fn month_of(word: &str) -> Option<u8> {
    let index = MONTHS
        .iter()
        .position(|month| *month == word || (word.len() == 3 && month.starts_with(word)))?;
    u8::try_from(index + 1).ok()
}

fn day_of(word: &str) -> Option<u8> {
    let digits = word.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let day: u8 = digits.parse().ok()?;
    (1..=31).contains(&day).then_some(day)
}

fn next_occurrence(month: u8, day: u8, today: CivilDate) -> Option<CivilDate> {
    [today.year, today.year + 1]
        .into_iter()
        .map(|year| CivilDate { year, month, day })
        .find(|date| valid(*date) && *date >= today)
}

fn valid(date: CivilDate) -> bool {
    (1..=12).contains(&date.month) && date.day >= 1 && date.day <= days_in_month(date)
}

fn days_in_month(date: CivilDate) -> u8 {
    match date.month {
        2 if date.year % 4 == 0 && (date.year % 100 != 0 || date.year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(date: CivilDate) -> i64 {
    let year = i64::from(date.year) - i64::from(date.month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(date.month);
    let day_of_year =
        (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(date.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> CivilDate {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    CivilDate {
        year: i32::try_from(year).unwrap_or(i32::MAX),
        month: u8::try_from(month).unwrap_or(1),
        day: u8::try_from(day).unwrap_or(1),
    }
}

fn add_days(date: CivilDate, days: i64) -> CivilDate {
    civil_from_days(days_from_civil(date) + days)
}

/// 0 is Monday. 1970-01-01 was a Thursday.
fn weekday_of(date: CivilDate) -> usize {
    usize::try_from((days_from_civil(date) + 3).rem_euclid(7)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use icalendar::{Calendar, CalendarComponent, Component};

    use super::{CivilDate, add_days, suggest_event, weekday_of};

    // Thursday.
    const TODAY: CivilDate = CivilDate {
        year: 2026,
        month: 10,
        day: 8,
    };

    fn start(text: &str) -> Option<String> {
        suggest_event(text, TODAY, "Meeting", "m1@mailune").map(|s| s.start)
    }

    #[test]
    fn a_sentence_with_a_date_and_a_time_becomes_an_ics_suggestion() {
        let suggestion = suggest_event(
            "Can we meet on October 15th at 3:30pm to go over the launch?",
            TODAY,
            "Launch review",
            "m1@mailune",
        )
        .unwrap();
        assert_eq!(suggestion.start, "20261015T153000");
        assert_eq!(suggestion.end, "20261015T163000");
        let calendar = Calendar::from_str(&suggestion.ics).unwrap();
        let Some(CalendarComponent::Event(event)) = calendar.iter().next() else {
            panic!("no event");
        };
        assert_eq!(event.get_summary(), Some("Launch review"));
        assert_eq!(event.get_uid(), Some("m1@mailune"));
        assert_eq!(event.property_value("DTSTART"), Some("20261015T153000"));
    }

    #[test]
    fn dates_and_times_in_other_shapes() {
        assert_eq!(
            start("Friday at 10 am works").as_deref(),
            Some("20261009T100000")
        );
        assert_eq!(start("Thursday 9am?").as_deref(), Some("20261015T090000"));
        assert_eq!(
            start("tomorrow at noon").as_deref(),
            Some("20261009T120000")
        );
        assert_eq!(
            start("2026-12-01 14:05 in room 4").as_deref(),
            Some("20261201T140500")
        );
        assert_eq!(start("3 Jan at 12am").as_deref(), Some("20270103T000000"));
        let late = suggest_event("today 11pm", TODAY, "x", "u").unwrap();
        assert_eq!(late.end, "20261009T000000");
        assert_eq!(start("see you on Friday"), None);
        assert_eq!(start("at 3pm"), None);
        assert_eq!(start("February 30 at 3pm"), None);
        assert_eq!(start("2026-10-20 at 13pm"), None);
    }

    #[test]
    fn civil_arithmetic_crosses_months_years_and_leap_days() {
        assert_eq!(weekday_of(TODAY), 3);
        let leap = CivilDate {
            year: 2028,
            month: 2,
            day: 28,
        };
        assert_eq!(add_days(leap, 1).day, 29);
        assert_eq!(
            add_days(leap, 2),
            CivilDate {
                year: 2028,
                month: 3,
                day: 1
            }
        );
        let eve = CivilDate {
            year: 2026,
            month: 12,
            day: 31,
        };
        assert_eq!(add_days(eve, 1).year, 2027);
    }
}
