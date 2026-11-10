use super::*;

fn dt(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> DateTime {
    DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

/// Год, месяц, день, часы, минуты, секунды
type Fields = (i32, u8, u8, u8, u8, u8);

// Пары «Unix-время - дата» сверены с `date -u -r <timestamp>` на macOS
const KNOWN: &[(i64, Fields)] = &[
    (0, (1970, 1, 1, 0, 0, 0)),
    (86399, (1970, 1, 1, 23, 59, 59)),
    (86400, (1970, 1, 2, 0, 0, 0)),
    (951782400, (2000, 2, 29, 0, 0, 0)),
    (951868800, (2000, 3, 1, 0, 0, 0)),
    (1709164800, (2024, 2, 29, 0, 0, 0)),
    (1793095200, (2026, 10, 27, 10, 0, 0)),
    (2147483647, (2038, 1, 19, 3, 14, 7)),
    (4102444800, (2100, 1, 1, 0, 0, 0)),
    (-1, (1969, 12, 31, 23, 59, 59)),
    (-86400, (1969, 12, 31, 0, 0, 0)),
    (-2208988800, (1900, 1, 1, 0, 0, 0)),
];

#[test_case]
fn test_from_unix_known_dates() {
    for &(timestamp, (y, mo, d, h, mi, s)) in KNOWN {
        assert_eq!(
            DateTime::from_unix(timestamp),
            dt(y, mo, d, h, mi, s),
            "{}",
            timestamp
        );
    }
}

#[test_case]
fn test_to_unix_known_dates() {
    for &(timestamp, (y, mo, d, h, mi, s)) in KNOWN {
        assert_eq!(dt(y, mo, d, h, mi, s).to_unix(), timestamp);
    }
}

#[test_case]
fn test_roundtrip_every_day_for_years() {
    // каждый день с 1968 по 2032 год, в том числе все 29 февраля
    let start = dt(1968, 1, 1, 12, 34, 56).to_unix();
    let mut previous = DateTime::from_unix(start - SECONDS_PER_DAY);
    for day in 0..(65 * 366) {
        let timestamp = start + day * SECONDS_PER_DAY;
        let date = DateTime::from_unix(timestamp);
        assert!(date.is_valid(), "{}", date);
        assert_eq!(date.to_unix(), timestamp);
        // следующий день - на единицу больше или первое число
        assert!(date.day == previous.day + 1 || date.day == 1, "{}", date);
        previous = date;
    }
}

#[test_case]
fn test_roundtrip_seconds_of_a_day() {
    let start = dt(2026, 10, 27, 0, 0, 0).to_unix();
    for second in (0..SECONDS_PER_DAY).step_by(37) {
        assert_eq!(
            DateTime::from_unix(start + second).to_unix(),
            start + second
        );
    }
}

#[test_case]
fn test_leap_years() {
    assert!(is_leap_year(2024));
    assert!(is_leap_year(2000));
    assert!(!is_leap_year(1900));
    assert!(!is_leap_year(2100));
    assert!(!is_leap_year(2026));
}

#[test_case]
fn test_days_in_month() {
    assert_eq!(days_in_month(2026, 1), 31);
    assert_eq!(days_in_month(2026, 2), 28);
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(days_in_month(1900, 2), 28);
    assert_eq!(days_in_month(2026, 4), 30);
    assert_eq!(days_in_month(2026, 12), 31);
}

#[test_case]
fn test_weekday() {
    assert_eq!(dt(1970, 1, 1, 0, 0, 0).weekday_name(), "Thu");
    assert_eq!(dt(1969, 12, 31, 0, 0, 0).weekday_name(), "Wed");
    assert_eq!(dt(2000, 1, 1, 0, 0, 0).weekday_name(), "Sat");
    assert_eq!(dt(2026, 10, 27, 0, 0, 0).weekday_name(), "Tue");
    // время суток день недели не меняет
    assert_eq!(dt(2026, 10, 27, 23, 59, 59).weekday(), 2);
}

#[test_case]
fn test_month_names() {
    assert_eq!(dt(2026, 1, 1, 0, 0, 0).month_name(), "Jan");
    assert_eq!(dt(2026, 10, 1, 0, 0, 0).month_name(), "Oct");
    assert_eq!(dt(2026, 12, 1, 0, 0, 0).month_name(), "Dec");
}

#[test_case]
fn test_is_valid() {
    assert!(dt(2024, 2, 29, 23, 59, 59).is_valid());
    assert!(!dt(2026, 2, 29, 0, 0, 0).is_valid());
    assert!(!dt(2026, 13, 1, 0, 0, 0).is_valid());
    assert!(!dt(2026, 0, 1, 0, 0, 0).is_valid());
    assert!(!dt(2026, 4, 31, 0, 0, 0).is_valid());
    assert!(!dt(2026, 1, 0, 0, 0, 0).is_valid());
    assert!(!dt(2026, 1, 1, 24, 0, 0).is_valid());
    assert!(!dt(2026, 1, 1, 0, 60, 0).is_valid());
    assert!(!dt(2026, 1, 1, 0, 0, 60).is_valid());
}

#[test_case]
fn test_display_iso() {
    use alloc::string::ToString;
    assert_eq!(dt(2026, 10, 7, 9, 5, 3).to_string(), "2026-10-07 09:05:03");
}
