#![no_std]
#![no_main]

mod common;

use core::panic::PanicInfo;
use rustos::{QemuExitCode, exit_qemu, serial_print, serial_println};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    rustos::init();
    stack_overflow();

    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    // volatile-чтение не даёт компилятору превратить рекурсию в цикл
    volatile::Volatile::new(0).read();
}

// Переполнение стека -> page fault на guard page -> double fault.
// CPU переключается на стек из IST и вызывает обработчик ядра, который паникует.
// Если стек из IST не настроен, CPU не сможет положить кадр прерывания
// на переполненный стек: triple fault, QEMU завершится и тест упадёт.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if common::panic_message_starts_with(info, "EXCEPTION: DOUBLE FAULT") {
        serial_println!("[ok]");
        exit_qemu(QemuExitCode::Success);
    } else {
        serial_println!("[failed]\n");
        serial_println!("Error: {}\n", info);
        exit_qemu(QemuExitCode::Failed);
    }
    loop {}
}
