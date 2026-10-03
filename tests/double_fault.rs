#![no_std]
#![no_main]

mod common;

use core::panic::PanicInfo;
use rustos::{QemuExitCode, exit_qemu, serial_print, serial_println};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("double_fault::page_fault_without_handler...\t");

    rustos::init();
    // адрес не отображён в память: page fault, обработчика для него нет,
    // поэтому CPU вызывает обработчик double fault
    unsafe {
        *(0xdeadbeef as *mut u8) = 42;
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
