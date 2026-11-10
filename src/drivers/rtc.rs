use crate::time::DateTime;
use x86_64::instructions::port::Port;

// RTC живёт в микросхеме CMOS вместе с небольшой памятью настроек BIOS.
// Доступ через два порта: в 0x70 пишется номер регистра, из 0x71 читается
// значение. Старший бит номера регистра отключает NMI, мы его не трогаем
const INDEX_PORT: u16 = 0x70;
const DATA_PORT: u16 = 0x71;

const SECONDS: u8 = 0x00;
const MINUTES: u8 = 0x02;
const HOURS: u8 = 0x04;
const DAY: u8 = 0x07;
const MONTH: u8 = 0x08;
const YEAR: u8 = 0x09;
const STATUS_A: u8 = 0x0a;
const STATUS_B: u8 = 0x0b;

/// STATUS A: RTC сейчас обновляет значения
const UPDATE_IN_PROGRESS: u8 = 1 << 7;
/// STATUS B: значения двоичные, а не BCD
pub const BINARY_MODE: u8 = 1 << 2;
/// STATUS B: часы в формате 24, а не 12
pub const HOUR_24: u8 = 1 << 1;
/// Старший бит часов в 12-часовом формате
pub const PM: u8 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registers {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    // две последние цифры года
    pub year: u8,
    pub status_b: u8,
}

fn read_register(register: u8) -> u8 {
    let mut index = Port::<u8>::new(INDEX_PORT);
    let mut data = Port::<u8>::new(DATA_PORT);
    unsafe {
        index.write(register);
        data.read()
    }
}

fn update_in_progress() -> bool {
    read_register(STATUS_A) & UPDATE_IN_PROGRESS != 0
}

fn read_registers() -> Registers {
    // ждем, пока кончится обновление
    while update_in_progress() {}
    Registers {
        second: read_register(SECONDS),
        minute: read_register(MINUTES),
        hour: read_register(HOURS),
        day: read_register(DAY),
        month: read_register(MONTH),
        year: read_register(YEAR),
        status_b: read_register(STATUS_B),
    }
}

pub fn read() -> DateTime {
    crate::arch::x86_64::without_interrupts(|| {
        let mut registers = read_registers();
        loop {
            let again = read_registers();
            if again == registers {
                return decode(registers);
            }
            registers = again;
        }
    })
}

pub fn bcd_to_binary(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

pub fn decode(registers: Registers) -> DateTime {
    let binary = registers.status_b & BINARY_MODE != 0;
    let hour_24 = registers.status_b & HOUR_24 != 0;
    let number = |value: u8| if binary { value } else { bcd_to_binary(value) };

    let pm = registers.hour & PM != 0;
    let mut hour = number(registers.hour & !PM);

    if !hour_24 {
        hour %= 12;
        if pm {
            hour += 12;
        }
    }
    DateTime {
        // в RTC только две цифры года; регистр века есть не везде,
        // поэтому считаем, что сейчас XXI век
        year: 2000 + number(registers.year) as i32,
        month: number(registers.month),
        day: number(registers.day),
        hour,
        minute: number(registers.minute),
        second: number(registers.second),
    }
}

#[cfg(test)]
mod tests;
