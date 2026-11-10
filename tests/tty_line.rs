#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Tty::new() + read_line на настоящем вводе: клавиатура PS/2 и терминал
// через COM1 (см. common::press_key и common::send_from_terminal)

extern crate alloc;

mod common;

use alloc::string::ToString;
use bootloader::{BootInfo, entry_point};
use common::{press_key, send_from_terminal};
use core::future::Future;
use core::panic::PanicInfo;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use rustos::tty::{ReadLine, Tty};
use spin::Mutex;

// скан-коды набора 1
const L: u8 = 0x26;
const S: u8 = 0x1f;
const ENTER: u8 = 0x1c;

const fn release(scancode: u8) -> u8 {
    scancode | 0x80
}

entry_point!(test_kernel_main);

// Tty::new() можно вызвать только один раз
static TTY: Mutex<Option<Tty>> = Mutex::new(None);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    rustos::init();
    common::init_heap(boot_info);
    *TTY.lock() = Some(Tty::new());

    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

/// Опрашивает future, пока он не завершится; между опросами CPU спит до
/// прерывания, как executor
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..1000 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        x86_64::instructions::hlt();
    }
    panic!("future did not complete");
}

/// Последняя строка экрана над курсором без пробелов в конце
fn row_above_cursor() -> ([u8; 80], usize) {
    const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;
    let mut row = [0; 80];
    x86_64::instructions::interrupts::without_interrupts(|| {
        for (col, byte) in row.iter_mut().enumerate() {
            *byte = unsafe { VGA_BUFFER.add(23 * 80 + col).read_volatile() } as u8;
        }
    });
    let len = row.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
    (row, len)
}

#[test_case]
fn test_line_from_terminal() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    send_from_terminal(b"echo hi\r");
    assert_eq!(
        block_on(tty.read_line("$ ")),
        ReadLine::Line("echo hi".to_string())
    );
    let (row, len) = row_above_cursor();
    assert_eq!(&row[..len], b"$ echo hi");
}

#[test_case]
fn test_line_from_keyboard() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    for scancode in [L, release(L), S, release(S), ENTER, release(ENTER)] {
        press_key(scancode);
    }
    assert_eq!(
        block_on(tty.read_line("$ ")),
        ReadLine::Line("ls".to_string())
    );
}

#[test_case]
fn test_line_typed_from_both_sources() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    press_key(L);
    press_key(release(L));
    // пока клавиша с клавиатуры не попала в строку, байты из терминала не шлём:
    // порядок между источниками не гарантирован
    let mut line = pin!(tty.read_line("$ "));
    let mut context = Context::from_waker(Waker::noop());
    assert!(line.as_mut().poll(&mut context).is_pending());
    send_from_terminal(b"s\r");
    let result = loop {
        if let Poll::Ready(result) = line.as_mut().poll(&mut context) {
            break result;
        }
        x86_64::instructions::hlt();
    };
    assert_eq!(result, ReadLine::Line("ls".to_string()));
}

#[test_case]
fn test_editing_from_terminal() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    // стрелка влево и вставка в середину, затем Backspace в конце
    send_from_terminal(b"helo\x1b[Dl\x1b[F!\x7f\r");
    assert_eq!(
        block_on(tty.read_line("$ ")),
        ReadLine::Line("hello".to_string())
    );
}

#[test_case]
fn test_history_from_terminal() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    send_from_terminal(b"pwd\r");
    block_on(tty.read_line("$ "));
    send_from_terminal(b"\x1b[A\r");
    assert_eq!(
        block_on(tty.read_line("$ ")),
        ReadLine::Line("pwd".to_string())
    );
}

#[test_case]
fn test_ctrl_c_from_terminal() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    send_from_terminal(b"oops\x03");
    assert_eq!(block_on(tty.read_line("$ ")), ReadLine::Interrupted);
    let (row, len) = row_above_cursor();
    assert_eq!(&row[..len], b"$ oops^C");
}

#[test_case]
fn test_ctrl_d_from_terminal() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    send_from_terminal(b"\x04");
    assert_eq!(block_on(tty.read_line("$ ")), ReadLine::Eof);
}

#[test_case]
fn test_unicode_from_terminal() {
    let mut tty = TTY.lock();
    let tty = tty.as_mut().unwrap();
    send_from_terminal("привет\r".as_bytes());
    assert_eq!(
        block_on(tty.read_line("$ ")),
        ReadLine::Line("привет".to_string())
    );
}
