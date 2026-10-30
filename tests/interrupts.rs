#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rustos::time::ticks;
use rustos::{print, println};
use x86_64::PrivilegeLevel;
use x86_64::instructions::segmentation::{CS, Segment};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    rustos::init();
    test_main();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

#[test_case]
fn test_breakpoint_exception() {
    // без обработчика int3 приводит к triple fault, QEMU (-no-reboot)
    // завершается с кодом, отличным от 33, и тест считается упавшим
    x86_64::instructions::interrupts::int3();
}

#[test_case]
fn test_breakpoint_exception_repeated() {
    // обработчик должен корректно возвращать управление каждый раз
    for _ in 0..10 {
        x86_64::instructions::interrupts::int3();
    }
}

#[test_case]
fn test_execution_continues_after_breakpoint() {
    let mut counter = 0;
    counter += 1;
    x86_64::instructions::interrupts::int3();
    counter += 1;
    assert_eq!(counter, 2);
}

#[test_case]
fn test_init_loads_gdt() {
    // GDT ядра: нулевой дескриптор, код ядра и TSS (две записи) - 4 * 8 байт
    let gdt = x86_64::instructions::tables::sgdt();
    assert_eq!(gdt.limit, 4 * 8 - 1);
}

#[test_case]
fn test_init_sets_kernel_code_segment() {
    let cs = CS::get_reg();
    assert_eq!(cs.index(), 1);
    assert_eq!(cs.rpl(), PrivilegeLevel::Ring0);
}

#[test_case]
fn test_init_enables_interrupts() {
    assert!(x86_64::instructions::interrupts::are_enabled());
}

#[test_case]
fn test_timer_interrupt_arrives() {
    // hlt ждёт следующего прерывания. Если PIC не настроен или таймер
    // замаскирован, тест зависнет и упадёт по test-timeout
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
}

#[test_case]
fn test_timer_interrupt_counts_ticks() {
    // других источников прерываний в тесте нет, поэтому каждый hlt
    // заканчивается прерыванием таймера
    let before = ticks();
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
    assert!(ticks() >= before + 3);
}

#[test_case]
fn test_timer_handler_prints_nothing() {
    // раньше обработчик таймера печатал '.', теперь только считает тики
    print!("\ntimer_marker");
    for _ in 0..3 {
        x86_64::instructions::hlt();
    }
    let (row, len) = last_vga_row();
    assert_eq!(&row[..len], b"timer_marker");
}

#[test_case]
fn test_println_no_deadlock_with_interrupts() {
    // println! вызывают и обработчики прерываний (breakpoint, page fault),
    // поэтому если он возьмёт WRITER с включёнными прерываниями,
    // рано или поздно будет deadlock
    for i in 0..1000 {
        println!("test_println_no_deadlock_with_interrupts {}", i);
        if i % 100 == 0 {
            x86_64::instructions::interrupts::int3();
        }
    }
}

/// Последняя строка экрана без пробелов в конце
fn last_vga_row() -> ([u8; 80], usize) {
    const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;
    let mut row = [0; 80];
    x86_64::instructions::interrupts::without_interrupts(|| {
        for (col, byte) in row.iter_mut().enumerate() {
            *byte = unsafe { VGA_BUFFER.add(24 * 80 + col).read_volatile() } as u8;
        }
    });
    let len = row.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
    (row, len)
}
