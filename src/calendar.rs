//! Small, dependency-free UTC calendar helpers for the metadata view.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

pub fn from_unix_seconds(seconds: u64) -> Date {
    let days = (seconds / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    Date { year, month, day }
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Monday = 0 through Sunday = 6.
pub fn weekday(year: i32, month: u32, day: u32) -> u32 {
    (days_from_civil(year, month, day) + 3).rem_euclid(7) as u32
}

pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year as i64 * 12 + month as i64 - 1 + delta as i64;
    let new_year = index.div_euclid(12) as i32;
    let new_month = index.rem_euclid(12) as u32 + 1;
    (new_year, new_month)
}

pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

// Howard Hinnant's civil calendar algorithms, with the Unix epoch as day 0.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64 - (month <= 2) as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let month = month as i64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + (month <= 2) as i64;
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_thursday_january_first() {
        assert_eq!(
            from_unix_seconds(0),
            Date {
                year: 1970,
                month: 1,
                day: 1
            }
        );
        assert_eq!(weekday(1970, 1, 1), 3);
    }

    #[test]
    fn month_lengths_and_leap_years_are_correct() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2025, 4), 30);
    }

    #[test]
    fn month_shifting_crosses_year_boundaries() {
        assert_eq!(shift_month(2025, 1, -1), (2024, 12));
        assert_eq!(shift_month(2025, 12, 1), (2026, 1));
    }
}
