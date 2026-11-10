#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Настоящий RTC QEMU. В тестах QEMU запускается с `-rtc base=2026-10-27T10:00:00`
// (см. test-args в Cargo.toml), поэтому дата известна заранее

use core::panic::PanicInfo;
use rustos::drivers::rtc;
use rustos::time::{self, TICKS_PER_SECOND};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    rustos::init();
    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

/// Ждёт, пока RTC перейдёт на следующую секунду, и возвращает тики в этот момент
fn wait_for_next_second() -> u64 {
    let start = rtc::read().second;
    while rtc::read().second == start {
        x86_64::instructions::hlt();
    }
    time::ticks()
}

#[test_case]
fn test_rtc_date_from_qemu() {
    let now = rtc::read();
    assert_eq!((now.year, now.month, now.day), (2026, 10, 27));
    assert_eq!(now.hour, 10);
    // с запуска QEMU прошло меньше пяти минут
    assert!(now.minute < 5, "{}", now);
}

#[test_case]
fn test_rtc_date_is_valid() {
    assert!(rtc::read().is_valid());
}

#[test_case]
fn test_two_reads_close() {
    let first = rtc::read().to_unix();
    let second = rtc::read().to_unix();
    assert!(second - first <= 1);
}

#[test_case]
fn test_rtc_second_advances() {
    let before = rtc::read().to_unix();
    wait_for_next_second();
    assert_eq!(rtc::read().to_unix(), before + 1);
}

#[test_case]
fn test_now_matches_rtc() {
    // init прочитал RTC один раз, дальше время считается по тикам
    let difference = time::now() - rtc::read().to_unix();
    assert!(difference.abs() <= 1, "difference {} s", difference);
}

#[test_case]
fn test_pit_frequency_matches_rtc() {
    // RTC - независимые часы: за 3 его секунды должно пройти 3 * 100 тиков
    let start = wait_for_next_second();
    for _ in 0..2 {
        wait_for_next_second();
    }
    let end = wait_for_next_second();
    let ticks = end - start;
    let expected = 3 * TICKS_PER_SECOND;
    assert!(
        ticks.abs_diff(expected) <= expected / 10,
        "{} ticks in 3 s",
        ticks
    );
}

#[test_case]
fn test_local_time_is_msk() {
    let utc = time::now_in(time::UTC);
    let local = time::now_in(time::LOCAL);
    assert_eq!(local.to_unix() - utc.to_unix(), 3 * 3600);
    assert_eq!(local.hour, utc.hour + 3);
}
