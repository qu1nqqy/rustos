pub use calendar::{DateTime, days_in_month, is_leap_year};
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use core::time::Duration;
pub use sleep::{MAX_SLEEPERS, Sleep, sleep, sleeping};
// TODO: timeout(duration, future) - результат future или None, если тот не успел
mod calendar;
mod sleep;

pub const TICKS_PER_SECOND: u64 = 100;

static TICKS: AtomicU64 = AtomicU64::new(0);

static BOOT_TIME: AtomicI64 = AtomicI64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeZone {
    pub name: &'static str,
    pub offset_seconds: i64,
}

pub const UTC: TimeZone = TimeZone {
    name: "UTC",
    offset_seconds: 0,
};

/// Часовой пояс для `date` и времени файлов. QEMU по умолчанию отдаёт в RTC
/// время UTC, поэтому местное время считаем сами
// TODO: изменяемый часовой пояс (Mutex или атомики) и команда `tz UTC` / `tz MSK`
pub const LOCAL: TimeZone = TimeZone {
    name: "MSK",
    offset_seconds: 3 * 60 * 60,
};

pub fn init() {
    crate::drivers::pit::init(TICKS_PER_SECOND as u32);
    set_time(crate::drivers::rtc::read().to_unix());
}

pub fn set_time(now: i64) {
    let boot_time = now - uptime().as_secs() as i64;
    BOOT_TIME.store(boot_time, Ordering::Relaxed);
}

pub fn now() -> i64 {
    BOOT_TIME.load(Ordering::Relaxed) + uptime().as_secs() as i64
}

pub fn now_in(tz: TimeZone) -> DateTime {
    DateTime::from_unix(now() + tz.offset_seconds)
}

pub(crate) fn tick() {
    let now = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    sleep::wake_expired(now);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn uptime() -> Duration {
    ticks_to_duration(ticks())
}

pub fn ticks_to_duration(ticks: u64) -> Duration {
    Duration::from_millis(ticks * 1000 / TICKS_PER_SECOND)
}

pub fn duration_to_ticks(duration: Duration) -> u64 {
    let millis = duration.as_millis() as u64;
    (millis * TICKS_PER_SECOND).div_ceil(1000)
}

#[cfg(test)]
mod tests;
