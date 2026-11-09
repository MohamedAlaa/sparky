use std::time::{Duration, Instant};

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

// Existing mailboxes contain clock labels, ISO dates, or RFC 2822 mail dates
// (with or without a leading weekday). Returns (year, month, day).
fn civil(value: &str) -> Option<(i64, i64, i64)> {
    let iso: Vec<_> = value.trim().split(['-', 'T', ' ']).take(3).collect();
    let (year, month, date): (i64, i64, i64) = if iso.len() == 3 && iso[0].len() == 4 && iso[0].bytes().all(|c| c.is_ascii_digit()) {
        (iso[0].parse().ok()?, iso[1].parse().ok()?, iso[2].parse().ok()?)
    } else {
        let parts: Vec<_> = value.trim_start_matches(|c: char| c.is_alphabetic() || c == ',' || c.is_whitespace()).split_whitespace().collect();
        if parts.len() < 3 { return None; }
        let month = MONTHS.iter().position(|m| parts[1].eq_ignore_ascii_case(m))? as i64 + 1;
        let year: i64 = parts[2].parse().ok()?;
        // RFC 2822 obsolete two-digit years.
        let year = match year { 0..=49 => year + 2000, 50..=99 => year + 1900, _ => year };
        (year, month, parts[0].parse().ok()?)
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&date) { return None; }
    Some((year, month, date))
}

// Days since 0000-03-01 (proleptic Gregorian).
fn day(value: &str) -> Option<i64> {
    let (year, month, date) = civil(value)?;
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let y = year - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    Some(era * 146097 + y * 365 + y / 4 - y / 100 + (153 * m + 2) / 5 + date - 1)
}

/// One display form for every provider date: "Sat, 7 Nov 2026".
/// Labels that aren't dates ("10:42 AM", "Yesterday") are returned unchanged.
pub fn normalize(value: &str) -> String {
    match (civil(value), day(value)) {
        // 0000-03-01 was a Wednesday.
        (Some((year, month, date)), Some(n)) => format!(
            "{}, {date} {} {year}",
            WEEKDAYS[(n + 3).rem_euclid(7) as usize],
            MONTHS[month as usize - 1]
        ),
        _ => value.trim().to_owned(),
    }
}

pub fn local_today() -> Option<i64> {
    // Cache the OS's local calendar date; refresh across midnight without spawning
    // a process on every frame. This also respects the user's local time zone.
    thread_local! {
        static CACHE: std::cell::RefCell<Option<(Instant, Option<i64>)>> = const { std::cell::RefCell::new(None) };
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(at, _)| at.elapsed() >= Duration::from_secs(60)) {
            let today = std::process::Command::new("/bin/date").arg("+%Y-%m-%d").output().ok()
                .filter(|output| output.status.success())
                .and_then(|output| day(String::from_utf8_lossy(&output.stdout).trim()));
            *cache = Some((Instant::now(), today));
        }
        cache.as_ref().and_then(|(_, today)| *today)
    })
}

pub fn group(value: &str, today: Option<i64>) -> usize {
    let label = value.trim().to_ascii_lowercase();
    if label == "yesterday" { return 1; }
    let clock = label.trim_end_matches("am").trim_end_matches("pm").trim();
    let is_clock = clock.split_once(':').is_some_and(|(h, m)|
        h.parse::<u8>().is_ok_and(|h| h < 24) && m.parse::<u8>().is_ok_and(|m| m < 60));
    if label == "today" || label == "now" || is_clock { return 0; }
    match (day(value), today) {
        (Some(date), Some(today)) if date == today => 0,
        (Some(date), Some(today)) if date == today - 1 => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_legacy_and_provider_dates_across_month_and_year_boundaries() {
        let today = day("2026-11-08");
        for value in ["Now", "9:41 AM", "Today", "2026-11-08", "Sun, 8 Nov 2026 "] {
            assert_eq!(group(value, today), 0, "{value}");
        }
        for value in ["Yesterday", "2026-11-07", "Sat, 7 Nov 2026 "] {
            assert_eq!(group(value, today), 1, "{value}");
        }
        for value in ["Tuesday", "2026-10-29", "", "unknown"] {
            assert_eq!(group(value, today), 2, "{value}");
        }
        assert_eq!(group("2025-12-31", day("2026-01-01")), 1);
        assert_eq!(group("2024-02-29", day("2024-03-01")), 1);
    }
    #[test]
    fn provider_dates_normalize_to_one_label() {
        // RFC 2822 with and without a weekday, a previously truncated label, ISO and two-digit years.
        for value in ["Sat, 7 Nov 2026 21:24:13 +0000", "07 Nov 2026 21:24:13 +0000", "07 Nov 2026 21:2",
                      "Sat, 7 Nov 2026 ", "2026-11-07", "2026-11-07T21:24:13Z", "7 Nov 26 21:24 GMT"] {
            assert_eq!(normalize(value), "Sat, 7 Nov 2026", "{value}");
        }
        assert_eq!(normalize("Sun, 1 Mar 2026"), "Sun, 1 Mar 2026");
        assert_eq!(normalize("29 Feb 2024 08:00:00 -0500"), "Thu, 29 Feb 2024");
        for label in ["10:42 AM", "Yesterday", "Tuesday", ""] {
            assert_eq!(normalize(label), label);
        }
    }
}
