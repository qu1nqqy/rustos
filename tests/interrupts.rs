#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

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
