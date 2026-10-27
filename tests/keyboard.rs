#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Нажатия клавиш эмулируются командой 0xD2 контроллера PS/2: она кладёт байт
// в выходной буфер контроллера так, будто его прислала клавиатура, и поднимает
// IRQ1. Дальше всё как с настоящей клавиатурой: обработчик ядра читает скан-код
// из порта 0x60 и кладёт его в очередь, задача print_keypresses достаёт его
// и печатает символ на экран, откуда тест его и читает.
//
// Задача опрашивается вручную (без executor'а): так тест сам решает, когда
// она выполняется, и видит, будит ли её прерывание

extern crate alloc;

mod common;

use alloc::boxed::Box;
use bootloader::{BootInfo, entry_point};
use core::future::Future;
use core::panic::PanicInfo;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use rustos::drivers::keyboard;
use rustos::println;
use spin::Mutex;
use x86_64::instructions::interrupts;
use x86_64::instructions::port::Port;

const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

const STATUS_OUTPUT_FULL: u8 = 1 << 0;
const STATUS_INPUT_FULL: u8 = 1 << 1;
const CMD_WRITE_KEYBOARD_OUTPUT: u8 = 0xd2;

// скан-коды набора 1: отпускание клавиши = код нажатия | 0x80
const A_PRESSED: u8 = 0x1e;
const A_RELEASED: u8 = 0x9e;
const B_PRESSED: u8 = 0x30;
const LSHIFT_PRESSED: u8 = 0x2a;
const LSHIFT_RELEASED: u8 = 0xaa;

entry_point!(test_kernel_main);

// ScancodeStream::new (внутри print_keypresses) можно вызвать только один раз,
// поэтому задача одна на все тесты
static PRINTER: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>> = Mutex::new(None);
static WOKEN: AtomicBool = AtomicBool::new(false);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    rustos::init();
    // print_keypresses создаёт очередь скан-кодов в куче
    common::init_heap(boot_info);

    *PRINTER.lock() = Some(Box::pin(keyboard::print_keypresses()));
    // первый опрос создаёт очередь; без неё обработчик выбрасывает скан-коды
    assert!(poll_printer().is_pending());

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

/// Опрашивает print_keypresses: она печатает все скан-коды из очереди
/// и возвращает Pending, когда очередь опустеет
fn poll_printer() -> Poll<()> {
    let waker = flag_waker();
    let mut context = Context::from_waker(&waker);
    let mut printer = PRINTER.lock();
    printer.as_mut().unwrap().as_mut().poll(&mut context)
}

fn status() -> u8 {
    unsafe { Port::<u8>::new(0x64).read() }
}

fn press(scancode: u8) {
    while status() & STATUS_INPUT_FULL != 0 {}
    unsafe { Port::<u8>::new(0x64).write(CMD_WRITE_KEYBOARD_OUTPUT) };
    while status() & STATUS_INPUT_FULL != 0 {}
    unsafe { Port::<u8>::new(0x60).write(scancode) };

    // буфер освободится, когда обработчик клавиатуры прочитает порт 0x60
    while status() & STATUS_OUTPUT_FULL != 0 {
        x86_64::instructions::hlt();
    }
}

/// Последняя строка экрана без точек от обработчика таймера и пробелов
fn last_row_without_timer_dots() -> ([u8; BUFFER_WIDTH], usize) {
    let mut out = [0; BUFFER_WIDTH];
    let mut len = 0;
    interrupts::without_interrupts(|| {
        for col in 0..BUFFER_WIDTH {
            let offset = (BUFFER_HEIGHT - 1) * BUFFER_WIDTH + col;
            let ch = unsafe { VGA_BUFFER.add(offset).read_volatile() } as u8;
            if ch != b'.' && ch != b' ' {
                out[len] = ch;
                len += 1;
            }
        }
    });
    (out, len)
}

fn assert_typed(scancodes: &[u8], expected: &[u8]) {
    // пустая строка, чтобы не зависеть от того, что уже напечатано
    println!();
    for &scancode in scancodes {
        press(scancode);
    }
    // задача бесконечная: она никогда не завершается
    assert!(poll_printer().is_pending());
    let (row, len) = last_row_without_timer_dots();
    assert_eq!(&row[..len], expected);
}

#[test_case]
fn test_key_press_prints_char() {
    assert_typed(&[A_PRESSED, A_RELEASED], b"a");
}

#[test_case]
fn test_key_release_prints_nothing() {
    assert_typed(&[A_RELEASED], b"");
}

#[test_case]
fn test_several_keys_in_order() {
    assert_typed(&[A_PRESSED, A_RELEASED, B_PRESSED, A_PRESSED], b"aba");
}

#[test_case]
fn test_shift_gives_uppercase() {
    // pc-keyboard отдаёт нажатие Shift как RawKey(LShift),
    // и обработчик печатает его название через {:?}
    assert_typed(
        &[LSHIFT_PRESSED, A_PRESSED, A_RELEASED, LSHIFT_RELEASED, A_PRESSED],
        b"LShiftAa",
    );
}

#[test_case]
fn test_scancodes_wait_for_task() {
    // обработчик прерывания только кладёт скан-код в очередь,
    // пока задача не опрошена, на экране ничего нет
    println!();
    press(A_PRESSED);
    let (_, len) = last_row_without_timer_dots();
    assert_eq!(len, 0);

    assert!(poll_printer().is_pending());
    let (row, len) = last_row_without_timer_dots();
    assert_eq!(&row[..len], b"a");
}

#[test_case]
fn test_key_press_wakes_task() {
    assert!(poll_printer().is_pending());
    WOKEN.store(false, Ordering::SeqCst);
    press(A_PRESSED);
    assert!(WOKEN.load(Ordering::SeqCst));
    assert!(poll_printer().is_pending());
}

#[test_case]
fn test_no_wake_without_key_press() {
    assert!(poll_printer().is_pending());
    WOKEN.store(false, Ordering::SeqCst);
    // ждём несколько прерываний таймера: они не должны будить задачу
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
    assert!(!WOKEN.load(Ordering::SeqCst));
}
