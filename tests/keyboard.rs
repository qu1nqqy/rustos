#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Нажатия клавиш эмулируются через контроллер PS/2 (см. common::press_key).
// Дальше всё как с настоящей клавиатурой: обработчик ядра читает скан-код
// из порта 0x60 и кладёт его в очередь, откуда его достаёт поток scancodes().
//
// Поток опрашивается вручную (без executor'а): так тест сам решает, когда
// его читать, и видит, будит ли его прерывание

extern crate alloc;

mod common;

use bootloader::{BootInfo, entry_point};
use common::press_key;
use core::panic::PanicInfo;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use futures_util::stream::Stream;
use rustos::drivers::keyboard;
use rustos::task::queue::QueueStream;
use spin::Mutex;

// скан-коды набора 1: отпускание клавиши = код нажатия | 0x80
const A_PRESSED: u8 = 0x1e;
const A_RELEASED: u8 = 0x9e;
const B_PRESSED: u8 = 0x30;

entry_point!(test_kernel_main);

// scancodes() можно вызвать только один раз, поэтому поток один на все тесты
static SCANCODES: Mutex<Option<QueueStream<u8>>> = Mutex::new(None);
static WOKEN: AtomicBool = AtomicBool::new(false);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    rustos::init();
    // очередь скан-кодов создаётся в куче
    common::init_heap(boot_info);
    *SCANCODES.lock() = Some(keyboard::scancodes());

    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

/// Waker, который только запоминает, что поток разбудили
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

fn poll_scancode() -> Poll<Option<u8>> {
    let waker = flag_waker();
    let mut context = Context::from_waker(&waker);
    let mut scancodes = SCANCODES.lock();
    Pin::new(scancodes.as_mut().unwrap()).poll_next(&mut context)
}

/// Читает всё, что накопилось в очереди
fn drain() -> ([u8; 16], usize) {
    let mut out = [0; 16];
    let mut len = 0;
    while let Poll::Ready(Some(scancode)) = poll_scancode() {
        out[len] = scancode;
        len += 1;
    }
    (out, len)
}

fn assert_scancodes(scancodes: &[u8]) {
    drain();
    for &scancode in scancodes {
        press_key(scancode);
    }
    let (out, len) = drain();
    assert_eq!(&out[..len], scancodes);
}

#[test_case]
fn test_key_press_reaches_stream() {
    assert_scancodes(&[A_PRESSED]);
}

#[test_case]
fn test_release_reaches_stream() {
    // драйвер не разбирает скан-коды, отпускание - такой же байт
    assert_scancodes(&[A_PRESSED, A_RELEASED]);
}

#[test_case]
fn test_scancodes_keep_order() {
    assert_scancodes(&[A_PRESSED, A_RELEASED, B_PRESSED, A_PRESSED]);
}

#[test_case]
fn test_empty_stream_is_pending() {
    drain();
    assert_eq!(poll_scancode(), Poll::Pending);
}

#[test_case]
fn test_key_press_wakes_stream() {
    drain();
    assert_eq!(poll_scancode(), Poll::Pending);
    WOKEN.store(false, Ordering::SeqCst);
    press_key(A_PRESSED);
    assert!(WOKEN.load(Ordering::SeqCst));
    assert_eq!(poll_scancode(), Poll::Ready(Some(A_PRESSED)));
}

#[test_case]
fn test_no_wake_without_key_press() {
    drain();
    assert_eq!(poll_scancode(), Poll::Pending);
    WOKEN.store(false, Ordering::SeqCst);
    // ждём несколько прерываний таймера: они не должны будить поток
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
    assert!(!WOKEN.load(Ordering::SeqCst));
}
