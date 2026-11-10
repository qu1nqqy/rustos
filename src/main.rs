#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustos::arch::x86_64::power;
use rustos::mm::allocator;
use rustos::mm::paging::{self, BootInfoFrameAllocator};
use rustos::println;
use rustos::shell::Shell;
use rustos::task::{Task, executor::Executor};
use rustos::tty::Tty;
use x86_64::VirtAddr;

// entry_point! проверяет сигнатуру kernel_main и сам объявляет _start,
// через который bootloader передаёт BootInfo
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // дублируем вывод в serial: с -serial stdio он попадает в терминал хоста
    rustos::console::enable_serial();
    rustos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { paging::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    #[cfg(test)]
    test_main();

    println!("rustos {}", env!("CARGO_PKG_VERSION"));
    println!("type 'help' to list commands");

    // TODO: вывести, сколько тиков прошло от начала kernel_main до этого места.
    // Почему почти всегда 0 или 1?
    let mut executor = Executor::new();
    executor.spawn(Task::new(shell()));
    executor.run();
}

async fn shell() {
    let mut shell = Shell::new(Tty::new());
    shell.run().await;
    println!("exit");
    power::poweroff();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rustos::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}
