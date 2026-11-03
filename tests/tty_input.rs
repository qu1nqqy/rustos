#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// tty::keys() целиком: клавиши приходят и с клавиатуры PS/2, и из терминала
// через COM1, в одном потоке и уже разобранными

extern crate alloc;

mod common;

use alloc::boxed::Box;
use bootloader::{BootInfo, entry_point};
use common::{press_key, send_from_terminal};
use core::panic::PanicInfo;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use futures_util::stream::Stream;
use rustos::tty::{self, Key};
use spin::Mutex;

// скан-коды набора 1
const A: u8 = 0x1e;
const EXTENDED: u8 = 0xe0;
const LEFT: u8 = 0x4b;
const LCTRL: u8 = 0x1d;
const C: u8 = 0x2e;

const fn release(scancode: u8) -> u8 {
    scancode | 0x80
}

entry_point!(test_kernel_main);

// keys() можно вызвать только один раз, поэтому поток один на все тесты
static KEYS: Mutex<Option<Pin<Box<dyn Stream<Item = Key> + Send>>>> = Mutex::new(None);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    rustos::init();
    common::init_heap(boot_info);
    *KEYS.lock() = Some(Box::pin(tty::keys()));

    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

fn poll_key() -> Poll<Option<Key>> {
    let mut context = Context::from_waker(Waker::noop());
    let mut keys = KEYS.lock();
    keys.as_mut().unwrap().as_mut().poll_next(&mut context)
}

/// Ждёт `n` клавиш из потока: байты из COM1 приходят с задержкой (см.
/// tests/serial_input.rs)
fn receive(n: usize) -> ([Option<Key>; 16], usize) {
    let mut keys = [None; 16];
    let mut len = 0;
    for _ in 0..1000 {
        while let Poll::Ready(Some(key)) = poll_key() {
            keys[len] = Some(key);
            len += 1;
        }
        if len >= n {
            return (keys, len);
        }
        x86_64::instructions::hlt();
    }
    panic!("received {} of {} keys", len, n);
}

fn assert_keys(keys: &[Option<Key>], expected: &[Key]) {
    assert_eq!(keys.len(), expected.len(), "keys: {:?}", keys);
    for (key, expected) in keys.iter().zip(expected) {
        assert_eq!(key.as_ref(), Some(expected));
    }
}

#[test_case]
fn test_key_from_keyboard() {
    press_key(A);
    press_key(release(A));
    let (keys, len) = receive(1);
    assert_keys(&keys[..len], &[Key::Char('a')]);
}

#[test_case]
fn test_special_keys_from_keyboard() {
    for scancode in [EXTENDED, LEFT, LCTRL, C, release(C), release(LCTRL)] {
        press_key(scancode);
    }
    let (keys, len) = receive(2);
    assert_keys(&keys[..len], &[Key::Left, Key::Ctrl('c')]);
}

#[test_case]
fn test_keys_from_terminal() {
    send_from_terminal(b"b\r");
    let (keys, len) = receive(2);
    assert_keys(&keys[..len], &[Key::Char('b'), Key::Enter]);
}

#[test_case]
fn test_special_keys_from_terminal() {
    send_from_terminal(b"\x1b[A\x7f\x03");
    let (keys, len) = receive(3);
    assert_keys(&keys[..len], &[Key::Up, Key::Backspace, Key::Ctrl('c')]);
}

#[test_case]
fn test_unicode_from_terminal() {
    send_from_terminal("ж".as_bytes());
    let (keys, len) = receive(1);
    assert_keys(&keys[..len], &[Key::Char('ж')]);
}

#[test_case]
fn test_both_sources_in_one_stream() {
    // порядок между источниками не гарантирован, внутри каждого - сохраняется
    press_key(A);
    press_key(release(A));
    send_from_terminal(b"xy");
    let (keys, len) = receive(3);
    assert_eq!(len, 3);

    let from_keyboard = keys[..len].iter().filter(|k| **k == Some(Key::Char('a')));
    assert_eq!(from_keyboard.count(), 1);
    let mut from_terminal = keys[..len].iter().filter(|k| **k != Some(Key::Char('a')));
    assert_eq!(from_terminal.next(), Some(&Some(Key::Char('x'))));
    assert_eq!(from_terminal.next(), Some(&Some(Key::Char('y'))));
}

#[test_case]
fn test_no_keys_without_input() {
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
    assert_eq!(poll_key(), Poll::Pending);
}
