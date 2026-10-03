#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
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
fn test_println_no_deadlock_with_timer() {
    // обработчик таймера печатает '.', поэтому если println! возьмёт
    // WRITER с включёнными прерываниями, рано или поздно будет deadlock
    for i in 0..1000 {
        rustos::println!("test_println_no_deadlock_with_timer {}", i);
    }
}
