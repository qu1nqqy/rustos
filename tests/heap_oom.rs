#![no_std]
#![no_main]

extern crate alloc;

mod common;

use alloc::vec::Vec;
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustos::mm::allocator::HEAP_SIZE;
use rustos::{QemuExitCode, exit_qemu, serial_print, serial_println};

entry_point!(test_kernel_main);

// Если памяти не хватило, alloc вызывает handle_alloc_error, который в no_std
// паникует с сообщением "memory allocation of N bytes failed"
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    serial_print!("heap_oom::allocation_larger_than_heap...\t");

    common::init_heap(boot_info);
    let vec: Vec<u8> = Vec::with_capacity(HEAP_SIZE + 1);
    // vec используется, чтобы аллокацию нельзя было выбросить при оптимизации
    serial_println!("[test did not panic] {:p}", vec.as_ptr());
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if common::panic_message_starts_with(info, "memory allocation of") {
        serial_println!("[ok]");
        exit_qemu(QemuExitCode::Success);
    } else {
        serial_println!("[failed]\n");
        serial_println!("Error: {}\n", info);
        exit_qemu(QemuExitCode::Failed);
    }
    loop {}
}
