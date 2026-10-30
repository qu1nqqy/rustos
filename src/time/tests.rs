use super::*;

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
