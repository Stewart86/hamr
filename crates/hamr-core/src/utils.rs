use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in milliseconds.
// u128 millis fits in u64 for realistic timestamps (until year 584942417)
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Convert epoch seconds to a date string "YYYY-MM-DD".
// u64 days since epoch fits in i64 for date calculations
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn date_string_from_epoch(secs: u64) -> String {
    let days = secs / 86400;
    let mut days = days as i64;
    let mut year = 1970i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0;
    for (i, &d) in days_in_months.iter().enumerate() {
        if days < d {
            month = i + 1;
            break;
        }
        days -= d;
    }

    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}")
}

pub(crate) fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub(crate) fn today_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    date_string_from_epoch(secs)
}

pub(crate) fn yesterday_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(86400);
    date_string_from_epoch(secs)
}
