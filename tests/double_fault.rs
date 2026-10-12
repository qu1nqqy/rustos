#![no_std]
#![no_main]

mod common;

use core::panic::PanicInfo;
use rustos::{QemuExitCode, exit_qemu, serial_print, serial_println};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("double_fault::divide_error_without_handler...\t");

    rustos::init();
    // деление на ноль: у divide error нет обработчика, при попытке его вызвать
    // CPU получает ещё одно исключение и вызывает обработчик double fault.
    // Page fault для этого не подходит: у него теперь есть свой обработчик.
    // Деление через asm, потому что в Rust деление на ноль - паника, а не исключение
    unsafe {
        core::arch::asm!(
            "xor edx, edx",
            "div ecx",
            in("ecx") 0,
            inout("eax") 1 => _,
            out("edx") _,
        );
    }

    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

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
