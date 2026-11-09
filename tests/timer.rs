#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Настоящий PIT после rustos::init(): частота и пробуждение спящих задач

extern crate alloc;

mod common;

use bootloader::{BootInfo, entry_point};
use core::future::Future;
use core::panic::PanicInfo;
use core::pin::pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::time::Duration;
use rustos::drivers::pit;
use rustos::time::{self, TICKS_PER_SECOND};
use x86_64::instructions::interrupts;
use x86_64::instructions::port::Port;

entry_point!(test_kernel_main);

static WOKEN: AtomicBool = AtomicBool::new(false);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    rustos::init();
    common::init_heap(boot_info);
    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

/// Waker, который только запоминает, что задачу разбудили
fn flag_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    fn wake(_: *const ()) {
        WOKEN.store(true, Ordering::SeqCst);
    }
    fn drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

    unsafe { Waker::from_raw(clone(core::ptr::null())) }
}

/// Текущее значение счётчика канала 0. Команда 0x00 «защёлкивает» счётчик,
/// чтобы его можно было прочитать двумя байтами, пока он продолжает считать
fn read_counter() -> u16 {
    let mut command = Port::<u8>::new(0x43);
    let mut data = Port::<u8>::new(0x40);
    interrupts::without_interrupts(|| unsafe {
        command.write(0x00);
        let low = data.read() as u16;
        let high = data.read() as u16;
        high << 8 | low
    })
}

#[test_case]
fn test_counter_runs_below_divisor() {
    // счётчик идёт от делителя вниз; с частотой по умолчанию (делитель 65536)
    // он почти всегда был бы больше
    let divisor = pit::divisor(TICKS_PER_SECOND as u32);
    for _ in 0..1000 {
        assert!(read_counter() <= divisor);
    }
}

#[test_case]
fn test_counter_changes() {
    let first = read_counter();
    let changed = (0..100_000).any(|_| read_counter() != first);
    assert!(changed);
}

#[test_case]
fn test_ticks_advance() {
    let before = time::ticks();
    for _ in 0..10 {
        x86_64::instructions::hlt();
    }
    assert!(time::ticks() >= before + 10);
}

#[test_case]
fn test_sleep_woken_by_timer_interrupt() {
    let start = time::ticks();
    let mut sleep = pin!(time::sleep(Duration::from_millis(50)));
    let waker = flag_waker();
    let mut context = Context::from_waker(&waker);
    WOKEN.store(false, Ordering::SeqCst);
    assert_eq!(sleep.as_mut().poll(&mut context), Poll::Pending);

    // спим, пока прерывание таймера не разбудит задачу
    while !WOKEN.load(Ordering::SeqCst) {
        x86_64::instructions::hlt();
    }
    assert_eq!(sleep.as_mut().poll(&mut context), Poll::Ready(()));
    // 50 мс - это 5 тиков
    assert!(time::ticks() >= start + 5);
    assert!(time::ticks() <= start + 7, "woken too late");
}

#[test_case]
fn test_sleeping_tasks_do_not_wake_early() {
    let mut sleep = pin!(time::sleep(Duration::from_millis(100)));
    let waker = flag_waker();
    let mut context = Context::from_waker(&waker);
    WOKEN.store(false, Ordering::SeqCst);
    assert_eq!(sleep.as_mut().poll(&mut context), Poll::Pending);
    for _ in 0..5 {
        x86_64::instructions::hlt();
    }
    assert!(!WOKEN.load(Ordering::SeqCst));
}

#[test_case]
fn test_uptime_matches_ticks() {
    let ticks = time::ticks();
    let uptime = time::uptime();
    assert!(uptime >= time::ticks_to_duration(ticks));
}
