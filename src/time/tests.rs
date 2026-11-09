use super::*;
use core::time::Duration;

// В unit-тестах прерывания выключены, поэтому TICKS меняет только сам тест

#[test_case]
fn test_tick_increments_by_one() {
    let before = ticks();
    tick();
    assert_eq!(ticks(), before + 1);
}

#[test_case]
fn test_ticks_do_not_change_without_tick() {
    let before = ticks();
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    assert_eq!(ticks(), before);
}

#[test_case]
fn test_many_ticks() {
    let before = ticks();
    for _ in 0..100 {
        tick();
    }
    assert_eq!(ticks(), before + 100);
}

#[test_case]
fn test_ticks_to_duration() {
    assert_eq!(ticks_to_duration(0), Duration::ZERO);
    assert_eq!(ticks_to_duration(1), Duration::from_millis(10));
    assert_eq!(ticks_to_duration(TICKS_PER_SECOND), Duration::from_secs(1));
    assert_eq!(ticks_to_duration(150), Duration::from_millis(1500));
}

#[test_case]
fn test_duration_to_ticks_rounds_up() {
    assert_eq!(duration_to_ticks(Duration::ZERO), 0);
    assert_eq!(duration_to_ticks(Duration::from_millis(10)), 1);
    // меньше тика - всё равно тик: спать нужно не меньше, чем просили
    assert_eq!(duration_to_ticks(Duration::from_millis(1)), 1);
    assert_eq!(duration_to_ticks(Duration::from_millis(15)), 2);
    assert_eq!(duration_to_ticks(Duration::from_secs(2)), 200);
}

#[test_case]
fn test_duration_ticks_roundtrip() {
    for ticks in [0, 1, 7, 100, 12345] {
        assert_eq!(duration_to_ticks(ticks_to_duration(ticks)), ticks);
    }
}

#[test_case]
fn test_uptime_follows_ticks() {
    let before = uptime();
    for _ in 0..TICKS_PER_SECOND {
        tick();
    }
    assert_eq!(uptime() - before, Duration::from_secs(1));
}
