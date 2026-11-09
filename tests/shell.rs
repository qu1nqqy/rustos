#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Оболочка на настоящем вводе: строки приходят из терминала через COM1
// (см. common::send_from_terminal). Каждый тест заканчивается командой exit,
// иначе Shell::run не вернётся

extern crate alloc;

mod common;

use bootloader::{BootInfo, entry_point};
use common::send_from_terminal;
use core::future::Future;
use core::panic::PanicInfo;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use rustos::shell::{STATUS_NOT_FOUND, Shell};
use rustos::tty::Tty;
use spin::Mutex;

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

/// Что осталось после сеанса оболочки
struct Session {
    status: i32,
    history_len: usize,
}

/// Отправляет строки из терминала и запускает оболочку до exit. Терминал
/// общий для всех тестов, поэтому после сеанса он возвращается в TTY
fn run_shell(lines: &[&[u8]]) -> Session {
    for line in lines {
        send_from_terminal(line);
    }
    let mut tty = TTY.lock();
    let mut shell = Shell::new(tty.take().unwrap());
    block_on(shell.run());
    assert!(shell.exit_requested());
    let session = Session {
        status: shell.status(),
        history_len: shell.history().len(),
    };
    *tty = Some(shell.into_tty());
    session
}

/// Строка экрана без пробелов в конце
fn screen_row(row: usize) -> ([u8; 80], usize) {
    const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;
    let mut out = [0; 80];
    x86_64::instructions::interrupts::without_interrupts(|| {
        for (col, byte) in out.iter_mut().enumerate() {
            *byte = unsafe { VGA_BUFFER.add(row * 80 + col).read_volatile() } as u8;
        }
    });
    let len = out.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
    (out, len)
}

/// Проверяет строки экрана над курсором, снизу вверх: `rows[0]` - строка
/// прямо над курсором
fn assert_rows_above_cursor(rows: &[&[u8]]) {
    for (i, expected) in rows.iter().enumerate() {
        let row = 23 - i;
        let (out, len) = screen_row(row);
        assert_eq!(&out[..len], *expected, "row {}", row);
    }
}

#[test_case]
fn test_command_output() {
    let session = run_shell(&[b"echo hello\r", b"exit\r"]);
    assert_eq!(session.status, 0);
    assert_rows_above_cursor(&[b"rustos$ exit", b"hello", b"rustos$ echo hello"]);
}

#[test_case]
fn test_unknown_command_and_exit_status() {
    let session = run_shell(&[b"nosuch\r", b"exit\r"]);
    assert_eq!(session.status, STATUS_NOT_FOUND);
    assert_rows_above_cursor(&[
        b"rustos$ exit",
        b"rustos: nosuch: command not found",
        b"rustos$ nosuch",
    ]);
}

#[test_case]
fn test_quotes_and_exit_code() {
    let session = run_shell(&[b"echo 'a  b'\r", b"exit 7\r"]);
    assert_eq!(session.status, 7);
    assert_rows_above_cursor(&[b"rustos$ exit 7", b"a  b", b"rustos$ echo 'a  b'"]);
}

#[test_case]
fn test_history_kept_between_sessions() {
    // история живёт в терминале, а он переходит от оболочки к оболочке
    // строки разные: повтор предыдущей строки в историю не попадает
    let before = run_shell(&[b"exit 0\r"]).history_len;
    let after = run_shell(&[b"echo x\r", b"exit\r"]).history_len;
    assert_eq!(after, before + 2);
}

#[test_case]
fn test_ctrl_c_then_command() {
    let session = run_shell(&[b"echo lost\x03", b"exit 1\r"]);
    assert_eq!(session.status, 1);
    assert_rows_above_cursor(&[b"rustos$ exit 1", b"rustos$ echo lost^C"]);
}

#[test_case]
fn test_sleep_waits() {
    let start = rustos::time::ticks();
    let session = run_shell(&[b"sleep 0.2\r", b"exit\r"]);
    assert_eq!(session.status, 0);
    // 200 мс - 20 тиков
    assert!(rustos::time::ticks() >= start + 20);
}
