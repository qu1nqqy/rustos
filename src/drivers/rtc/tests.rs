use super::*;

// Чтение настоящих регистров проверяется в tests/rtc.rs, здесь - разбор

fn registers(status_b: u8, hour: u8) -> Registers {
    Registers {
        second: 0x05,
        minute: 0x30,
        hour,
        day: 0x27,
        month: 0x10,
        year: 0x26,
        status_b,
    }
}

#[test_case]
fn test_bcd_to_binary() {
    assert_eq!(bcd_to_binary(0x00), 0);
    assert_eq!(bcd_to_binary(0x09), 9);
    assert_eq!(bcd_to_binary(0x10), 10);
    assert_eq!(bcd_to_binary(0x59), 59);
    assert_eq!(bcd_to_binary(0x99), 99);
}

#[test_case]
fn test_decode_bcd_24h() {
    // так RTC настроен в QEMU: BCD и 24 часа
    let date = decode(registers(HOUR_24, 0x13));
    assert_eq!(date.to_string_parts(), (2026, 10, 27, 13, 30, 5));
}

#[test_case]
fn test_decode_binary_24h() {
    let raw = Registers {
        second: 5,
        minute: 30,
        hour: 13,
        day: 27,
        month: 10,
        year: 26,
        status_b: BINARY_MODE | HOUR_24,
    };
    assert_eq!(decode(raw).to_string_parts(), (2026, 10, 27, 13, 30, 5));
}

#[test_case]
fn test_decode_12h_pm() {
    assert_eq!(decode(registers(0, PM | 0x01)).hour, 13);
    assert_eq!(decode(registers(0, PM | 0x11)).hour, 23);
}

#[test_case]
fn test_decode_12h_am() {
    assert_eq!(decode(registers(0, 0x01)).hour, 1);
    assert_eq!(decode(registers(0, 0x11)).hour, 11);
}

#[test_case]
fn test_decode_12h_midnight_and_noon() {
    // 12 AM - полночь, 12 PM - полдень
    assert_eq!(decode(registers(0, 0x12)).hour, 0);
    assert_eq!(decode(registers(0, PM | 0x12)).hour, 12);
}

#[test_case]
fn test_decode_12h_binary() {
    assert_eq!(decode(registers(BINARY_MODE, PM | 11)).hour, 23);
    assert_eq!(decode(registers(BINARY_MODE, 12)).hour, 0);
}

#[test_case]
fn test_decode_year_in_21st_century() {
    let mut raw = registers(HOUR_24, 0);
    raw.year = 0x00;
    assert_eq!(decode(raw).year, 2000);
    raw.year = 0x99;
    assert_eq!(decode(raw).year, 2099);
}

trait Parts {
    fn to_string_parts(&self) -> (i32, u8, u8, u8, u8, u8);
}

impl Parts for DateTime {
    fn to_string_parts(&self) -> (i32, u8, u8, u8, u8, u8) {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        )
    }
}
