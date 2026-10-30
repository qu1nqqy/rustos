use super::*;
use crate::serial_println;

// Внутренности VGA-буфера приватны для drivers::vga, поэтому экран читается
// напрямую из видеопамяти, как в tests/keyboard.rs

const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// Последняя строка экрана без пробелов в конце
fn last_row() -> ([u8; BUFFER_WIDTH], usize) {
    let mut row = [0; BUFFER_WIDTH];
    for (col, byte) in row.iter_mut().enumerate() {
        let offset = (BUFFER_HEIGHT - 1) * BUFFER_WIDTH + col;
        *byte = unsafe { VGA_BUFFER.add(offset).read_volatile() } as u8;
    }
    let len = row.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
    (row, len)
}

fn assert_last_row(expected: &[u8]) {
    let (row, len) = last_row();
    assert_eq!(&row[..len], expected);
}

#[test_case]
fn test_serial_disabled_by_default() {
    // в тестах serial занят отчётом тест-раннера
    assert!(!is_serial_enabled());
}

#[test_case]
fn test_enable_and_disable_serial() {
    enable_serial();
    assert!(is_serial_enabled());
    disable_serial();
    assert!(!is_serial_enabled());
}

#[test_case]
fn test_println_writes_to_vga() {
    println!("\nconsole_vga_output");
    // после println! курсор уже на новой пустой строке
    assert_last_row(b"");
    print!("\nconsole_vga_output");
    assert_last_row(b"console_vga_output");
}

#[test_case]
fn test_print_continues_row() {
    print!("\nab");
    print!("cd");
    assert_last_row(b"abcd");
}

#[test_case]
fn test_println_without_args_ends_line() {
    print!("\nsome text");
    println!();
    assert_last_row(b"");
}

#[test_case]
fn test_println_formats_arguments() {
    print!("\n{} + {} = {}", 2, 2, 4);
    assert_last_row(b"2 + 2 = 4");
}

#[test_case]
fn test_output_with_serial_enabled_still_goes_to_vga() {
    // строка уйдёт и в serial, то есть попадёт в вывод cargo test
    enable_serial();
    print!("\nconsole_serial_output");
    disable_serial();
    serial_println!();
    assert_last_row(b"console_serial_output");
}
