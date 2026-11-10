use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub year: i32,
    // 1..=12
    pub month: u8,
    // 1..=31,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Номер дня от 1970-01-01 (может быть отрицательным).
///
/// Алгоритм Говарда Хиннанта (`days_from_civil`): год начинается с 1 марта,
/// тогда 29 февраля - последний день года, и длины месяцев с марта по январь
/// укладываются в формулу `(153 * m + 2) / 5`
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let (month, day) = (month as i64, day as i64);
    // январь и февраль считаются последними месяцами предыдущего года
    let year = year as i64 - if month <= 2 { 1 } else { 0 };
    // эра - 400 лет: столько длится полный цикл високосных лет григорианского календаря
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    // 719468 - номер дня 1970-01-01 от 0000-03-01
    era * 146097 + day_of_era - 719468
}

/// Обратное к `days_from_civil`: год, месяц и день по номеру дня от 1970-01-01
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let days = days + 719468;
    let era = days.div_euclid(146097);
    let day_of_era = days.rem_euclid(146097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u8, day as u8)
}

impl DateTime {
    pub fn from_unix(timestamp: i64) -> DateTime {
        let days = timestamp.div_euclid(SECONDS_PER_DAY);
        let seconds = timestamp.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        DateTime {
            year,
            month,
            day,
            hour: (seconds / (60 * 60)) as u8,
            minute: (seconds / 60 % 60) as u8,
            second: (seconds % 60) as u8,
        }
    }

    pub fn to_unix(&self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        let seconds = self.hour as i64 * 3600 + self.minute as i64 * 60 + self.second as i64;
        days * SECONDS_PER_DAY + seconds
    }

    pub fn weekday(&self) -> u8 {
        let days = days_from_civil(self.year, self.month, self.day);
        (days + 4).rem_euclid(7) as u8
    }

    pub fn is_valid(&self) -> bool {
        (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }

    pub fn weekday_name(&self) -> &'static str {
        WEEKDAYS[self.weekday() as usize]
    }

    pub fn month_name(&self) -> &'static str {
        MONTHS[self.month as usize - 1]
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

#[cfg(test)]
mod tests;
