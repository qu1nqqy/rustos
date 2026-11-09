#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustos::mm::allocator;
use rustos::mm::paging::{self, BootInfoFrameAllocator};
use rustos::println;
use rustos::task::{Task, executor::Executor};
use rustos::tty::{ReadLine, Tty};
use x86_64::VirtAddr;

// entry_point! проверяет сигнатуру kernel_main и сам объявляет _start,
// через который bootloader передаёт BootInfo
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // дублируем вывод в serial: с -serial stdio он попадает в терминал хоста
    rustos::console::enable_serial();
    println!("Hello World{}", "!");
    rustos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { paging::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(echo_lines()));
    executor.run();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

/// Читает строки и повторяет их, пока не нажат Ctrl+D
async fn echo_lines() {
    let mut tty = Tty::new();
    loop {
        match tty.read_line("> ").await {
            ReadLine::Line(line) => println!("you typed: {}", line),
            ReadLine::Interrupted => {}
            ReadLine::Eof => break,
        }
    }
    println!("bye");
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
