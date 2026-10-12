#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustos::memory::{self, BootInfoFrameAllocator};
use rustos::println;
use x86_64::{VirtAddr, structures::paging::Page};

// entry_point! проверяет сигнатуру kernel_main и сам объявляет _start,
// через который bootloader передаёт BootInfo
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World{}", "!");
    rustos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // пример из статьи: отображаем страницу 0 на VGA-буфер и пишем через неё
    // "New!" (чёрным по белому) в строку 400 * 8 / 160 = 20 экрана
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe {
        page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e);
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    rustos::hlt_loop();
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
