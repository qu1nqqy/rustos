#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

// bootloader должен линковаться и в образы из tests/, которые не используют
// entry_point! и объявляют _start сами
extern crate bootloader as _;
// крейт alloc (Box, Vec, Rc...) не подключается в no_std автоматически
extern crate alloc;

pub mod arch;
pub mod shell;
pub mod console;
pub mod drivers;
pub mod mm;
pub mod task;
pub mod time;
pub mod tty;

pub use arch::x86_64::hlt_loop;

#[cfg(test)]
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

pub fn init() {
    drivers::serial::init();
    arch::x86_64::init();
}

// Инфраструктура для тестов, общая для unit- и интеграционных тестов

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

// Точка входа для `cargo test --lib`. entry_point! проверяет сигнатуру
// функции и сам объявляет _start
#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    // init() здесь не вызывается: unit-тесты не меняют состояние CPU,
    // загрузка GDT/IDT и прерывания проверяются в tests/ (см. testing.md).
    // Куча нужна коду, который работает с Box, Vec и String
    let phys_mem_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { mm::paging::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { mm::paging::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    mm::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[cfg(test)]
mod tests;
