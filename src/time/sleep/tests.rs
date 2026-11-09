use super::*;
use crate::time::tick;
use core::pin::pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{RawWaker, RawWakerVTable};

// В unit-тестах прерывания выключены: тики добавляет сам тест через tick(),
// и так же, как обработчик таймера, tick() будит спящих

static WAKES: AtomicUsize = AtomicUsize::new(0);

/// Waker, который считает пробуждения
fn counting_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    fn wake(_: *const ()) {
        WAKES.fetch_add(1, Ordering::SeqCst);
    }
    fn drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

    unsafe { Waker::from_raw(clone(core::ptr::null())) }
}

fn poll(sleep: Pin<&mut Sleep>) -> Poll<()> {
    let waker = counting_waker();
    let mut context = Context::from_waker(&waker);
    sleep.poll(&mut context)
}

fn ticks_for(n: u64) {
    for _ in 0..n {
        tick();
    }
}

#[test_case]
fn test_zero_sleep_is_ready_at_once() {
    let mut sleep = pin!(sleep(Duration::ZERO));
    assert_eq!(poll(sleep.as_mut()), Poll::Ready(()));
}

#[test_case]
fn test_sleep_pending_until_deadline() {
    let mut sleep = pin!(sleep(Duration::from_millis(30)));
    for _ in 0..3 {
        assert_eq!(poll(sleep.as_mut()), Poll::Pending);
        tick();
    }
    assert_eq!(poll(sleep.as_mut()), Poll::Ready(()));
}

#[test_case]
fn test_ready_sleep_stays_ready() {
    let mut sleep = pin!(sleep(Duration::from_millis(10)));
    assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    tick();
    assert_eq!(poll(sleep.as_mut()), Poll::Ready(()));
    assert_eq!(poll(sleep.as_mut()), Poll::Ready(()));
}

#[test_case]
fn test_tick_wakes_sleeper_at_deadline() {
    let mut sleep = pin!(sleep(Duration::from_millis(20)));
    assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    let before = WAKES.load(Ordering::SeqCst);
    tick();
    assert_eq!(WAKES.load(Ordering::SeqCst), before);
    tick();
    assert_eq!(WAKES.load(Ordering::SeqCst), before + 1);
}

#[test_case]
fn test_sleeper_woken_only_once() {
    let mut sleep = pin!(sleep(Duration::from_millis(10)));
    assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    let before = WAKES.load(Ordering::SeqCst);
    ticks_for(5);
    assert_eq!(WAKES.load(Ordering::SeqCst), before + 1);
}

#[test_case]
fn test_sleeper_not_polled_is_not_registered() {
    // Sleep ничего не делает, пока его не опросили, как любой future
    let before = sleeping();
    let _sleep = sleep(Duration::from_secs(1));
    assert_eq!(sleeping(), before);
}

#[test_case]
fn test_pending_sleep_takes_slot() {
    let before = sleeping();
    let mut sleep = pin!(sleep(Duration::from_secs(1)));
    assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    assert_eq!(sleeping(), before + 1);
    // повторный опрос не занимает вторую ячейку
    assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    assert_eq!(sleeping(), before + 1);
}

#[test_case]
fn test_finished_sleep_frees_slot() {
    let before = sleeping();
    let mut sleep = pin!(sleep(Duration::from_millis(10)));
    assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    tick();
    assert_eq!(poll(sleep.as_mut()), Poll::Ready(()));
    assert_eq!(sleeping(), before);
}

#[test_case]
fn test_dropped_sleep_frees_slot() {
    let before = sleeping();
    {
        let mut sleep = pin!(sleep(Duration::from_secs(1)));
        assert_eq!(poll(sleep.as_mut()), Poll::Pending);
        assert_eq!(sleeping(), before + 1);
    }
    assert_eq!(sleeping(), before);
}

#[test_case]
fn test_sleepers_wake_in_deadline_order() {
    let mut short = pin!(sleep(Duration::from_millis(10)));
    let mut long = pin!(sleep(Duration::from_millis(30)));
    assert_eq!(poll(long.as_mut()), Poll::Pending);
    assert_eq!(poll(short.as_mut()), Poll::Pending);
    tick();
    assert_eq!(poll(short.as_mut()), Poll::Ready(()));
    assert_eq!(poll(long.as_mut()), Poll::Pending);
    ticks_for(2);
    assert_eq!(poll(long.as_mut()), Poll::Ready(()));
}

#[test_case]
fn test_all_slots_can_be_used() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    let free = MAX_SLEEPERS - sleeping();
    let mut sleeps: Vec<Pin<Box<Sleep>>> = (0..free)
        .map(|_| Box::pin(sleep(Duration::from_secs(1))))
        .collect();
    for sleep in &mut sleeps {
        assert_eq!(poll(sleep.as_mut()), Poll::Pending);
    }
    assert_eq!(sleeping(), MAX_SLEEPERS);
    drop(sleeps);
    assert_eq!(sleeping(), MAX_SLEEPERS - free);
}
