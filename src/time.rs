use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;
pub use sleep::{MAX_SLEEPERS, Sleep, sleep, sleeping};

mod sleep;

pub const TICKS_PER_SECOND: u64 = 100;

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    crate::drivers::pit::init(TICKS_PER_SECOND as u32);
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
