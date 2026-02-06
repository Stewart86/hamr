use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const SECS_PER_DAY: u64 = 86_400;
pub(crate) const SECS_PER_HOUR: u64 = 3_600;

/// Get current timestamp in milliseconds.
// u128 millis fits in u64 for realistic timestamps (until year 584942417)
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Extract hour (0-23) and weekday (0=Mon, 6=Sun) from epoch seconds.
// Time math: u64 secs -> u32 hour, usize weekday
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn time_components_from_epoch(epoch_secs: u64) -> (u32, usize) {
    let days_since_epoch = epoch_secs / SECS_PER_DAY;
    let weekday = ((days_since_epoch + 3) % 7) as usize; // 0=Mon, 6=Sun (epoch was Thursday)
    let secs_today = epoch_secs % SECS_PER_DAY;
    let hour = (secs_today / SECS_PER_HOUR) as u32;
    (hour, weekday)
}

/// Convert epoch seconds to a date string "YYYY-MM-DD".
// u64 days since epoch fits in i64 for date calculations
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn date_string_from_epoch(secs: u64) -> String {
    let days = secs / SECS_PER_DAY;
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

    // If the loop exhausted all months without breaking, we're in December
    if month == 0 {
        month = 12;
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
        .saturating_sub(SECS_PER_DAY);
    date_string_from_epoch(secs)
}
