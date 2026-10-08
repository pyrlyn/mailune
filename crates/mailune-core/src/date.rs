//! RFC 3339 timestamps as JMAP (`receivedAt`) and Microsoft Graph
//! (`receivedDateTime`) send them, turned into seconds since the epoch so
//! the store can order by arrival.

/// Seconds since the Unix epoch for `2026-03-01T12:00:00Z`, with optional
/// fractional seconds and a `Z` or `±HH:MM` offset. `None` for anything else.
pub fn parse_rfc3339(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[13] != b':' {
        return None;
    }
    if !matches!(bytes[10], b'T' | b't' | b' ') || bytes[16] != b':' {
        return None;
    }
    let year = number(&text[0..4])?;
    let month = number(&text[5..7])?;
    let day = number(&text[8..10])?;
    let hour = number(&text[11..13])?;
    let minute = number(&text[14..16])?;
    let second = number(&text[17..19])?;
    if !(1..=12).contains(&month) || day == 0 || day > 31 || hour > 23 || minute > 59 {
        return None;
    }
    // 60 is a leap second; it is folded into the next minute.
    if second > 60 {
        return None;
    }
    let mut rest = &text[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        rest = &fraction[digits..];
    }
    let offset = match rest {
        "Z" | "z" => 0,
        _ => {
            let sign = match rest.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            if rest.len() != 6 || rest.as_bytes()[3] != b':' {
                return None;
            }
            sign * (number(&rest[1..3])? * 3600 + number(&rest[4..6])? * 60)
        }
    };
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

fn number(text: &str) -> Option<i64> {
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

/// Howard Hinnant's days-from-civil: proleptic Gregorian, day 0 = 1970-01-01.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::parse_rfc3339;

    #[test]
    fn utc_offsets_and_fractions_parse() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("2026-03-01T12:00:00Z"), Some(1_772_366_400));
        assert_eq!(
            parse_rfc3339("2026-03-01T12:00:00.1234567Z"),
            Some(1_772_366_400)
        );
        assert_eq!(
            parse_rfc3339("2026-03-01T14:30:00+02:30"),
            Some(1_772_366_400)
        );
        assert_eq!(parse_rfc3339("2000-02-29T00:00:00Z"), Some(951_782_400));
    }

    #[test]
    fn malformed_stamps_are_none() {
        for bad in [
            "",
            "2026-03-01",
            "2026-13-01T00:00:00Z",
            "2026-03-01T24:00:00Z",
            "2026-03-01T12:00:00",
            "2026-03-01T12:00:00.Z",
            "2026-03-01T12:00:00+0200",
            "20x6-03-01T12:00:00Z",
        ] {
            assert_eq!(parse_rfc3339(bad), None, "{bad}");
        }
    }
}
