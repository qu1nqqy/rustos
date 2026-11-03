#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Байты «из терминала» отправляются через loopback UART (см.
// common::send_from_terminal): они проходят тот же путь, что и настоящие -
// прерывание IRQ4, обработчик, очередь и поток drivers::serial::input()

extern crate alloc;

mod common;

use bootloader::{BootInfo, entry_point};
use common::{SERIAL_CHUNK, send_from_terminal};
use core::panic::PanicInfo;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use futures_util::stream::Stream;
use rustos::drivers::serial;
use rustos::task::queue::QueueStream;
use spin::Mutex;

entry_point!(test_kernel_main);

// input() можно вызвать только один раз, поэтому поток один на все тесты
static INPUT: Mutex<Option<QueueStream<u8>>> = Mutex::new(None);
static WOKEN: AtomicBool = AtomicBool::new(false);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    rustos::init();
    common::init_heap(boot_info);
    *INPUT.lock() = Some(serial::input());

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

fn poll_input() -> Poll<Option<u8>> {
    let waker = flag_waker();
    let mut context = Context::from_waker(&waker);
    let mut input = INPUT.lock();
    Pin::new(input.as_mut().unwrap()).poll_next(&mut context)
}

/// Ждёт `n` байтов из потока. UART с FIFO поднимает прерывание не на каждый
/// байт, а когда FIFO наполнится или после паузы в несколько символов,
/// поэтому байты приходят не сразу
fn receive(out: &mut [u8]) {
    let mut len = 0;
    for _ in 0..1000 {
        while let Poll::Ready(Some(byte)) = poll_input() {
            out[len] = byte;
            len += 1;
        }
        if len >= out.len() {
            return;
        }
        x86_64::instructions::hlt();
    }
    panic!("received {} of {} bytes", len, out.len());
}

fn assert_received(bytes: &[u8]) {
    let mut out = [0; 64];
    let out = &mut out[..bytes.len()];
    for chunk in bytes.chunks(SERIAL_CHUNK) {
        send_from_terminal(chunk);
    }
    receive(out);
    assert_eq!(out, bytes);
}

#[test_case]
fn test_byte_reaches_stream() {
    assert_received(b"a");
}

#[test_case]
fn test_bytes_keep_order() {
    assert_received(b"ls -l\r");
}

#[test_case]
fn test_raw_bytes_pass_unchanged() {
    // драйвер не разбирает байты: escape-последовательности и UTF-8 - дело tty
    assert_received(b"\x1b[A\x7f\x03");
    assert_received("ж".as_bytes());
}

#[test_case]
fn test_full_fifo_at_once() {
    assert_received(b"0123456789abcdef");
}

#[test_case]
fn test_many_bytes_in_chunks() {
    assert_received(b"the quick brown fox jumps over the lazy dog 0123456789");
}

#[test_case]
fn test_empty_stream_is_pending() {
    assert_eq!(poll_input(), Poll::Pending);
}

#[test_case]
fn test_received_byte_wakes_stream() {
    assert_eq!(poll_input(), Poll::Pending);
    WOKEN.store(false, Ordering::SeqCst);
    send_from_terminal(b"x");
    for _ in 0..1000 {
        if WOKEN.load(Ordering::SeqCst) {
            break;
        }
        x86_64::instructions::hlt();
    }
    assert!(WOKEN.load(Ordering::SeqCst));
    assert_eq!(poll_input(), Poll::Ready(Some(b'x')));
}

#[test_case]
fn test_no_wake_without_input() {
    assert_eq!(poll_input(), Poll::Pending);
    WOKEN.store(false, Ordering::SeqCst);
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
    assert!(!WOKEN.load(Ordering::SeqCst));
}

#[test_case]
fn test_serial_output_still_works() {
    // после loopback вывод снова идёт в терминал
    send_from_terminal(b"y");
    let mut out = [0; 1];
    receive(&mut out);
    rustos::serial_println!("serial output after loopback");
}
