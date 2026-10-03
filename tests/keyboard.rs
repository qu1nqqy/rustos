#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Нажатия клавиш эмулируются командой 0xD2 контроллера PS/2: она кладёт байт
// в выходной буфер контроллера так, будто его прислала клавиатура, и поднимает
// IRQ1. Дальше всё как с настоящей клавиатурой: обработчик ядра читает скан-код
// из порта 0x60 и печатает символ на экран, откуда тест его и читает.

use core::panic::PanicInfo;
use rustos::println;
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

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    rustos::init();
    test_main();

    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
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
