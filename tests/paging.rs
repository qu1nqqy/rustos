#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::bootinfo::MemoryRegionType;
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustos::memory::{self, BootInfoFrameAllocator};
use spin::{Mutex, Once};
use x86_64::structures::paging::{
    FrameAllocator, OffsetPageTable, Page, PhysFrame, Size4KiB, Translate,
};
use x86_64::{PhysAddr, VirtAddr};

// Тесты читают и меняют настоящие таблицы страниц, поэтому они в отдельном
// образе. rustos::init() не вызывается: без обработчика page fault ошибка
// в тесте сразу даст triple fault и быстрый провал, а не зависание в hlt_loop

entry_point!(test_kernel_main);

static BOOT_INFO: Once<&'static BootInfo> = Once::new();
// memory::init можно вызвать только один раз, поэтому mapper и аллокатор
// создаются в test_kernel_main и общие для всех тестов
static PAGING: Mutex<Option<(OffsetPageTable<'static>, BootInfoFrameAllocator)>> =
    Mutex::new(None);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    BOOT_INFO.call_once(|| boot_info);

    let mapper = unsafe { memory::init(phys_mem_offset()) };
    let frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    *PAGING.lock() = Some((mapper, frame_allocator));

    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

fn boot_info() -> &'static BootInfo {
    BOOT_INFO.r#try().expect("BOOT_INFO не инициализирован")
}

fn phys_mem_offset() -> VirtAddr {
    VirtAddr::new(boot_info().physical_memory_offset)
}

fn translate(addr: u64) -> Option<PhysAddr> {
    unsafe { memory::translate_addr(VirtAddr::new(addr), phys_mem_offset()) }
}

#[test_case]
fn test_translate_vga_buffer_identity_mapped() {
    // bootloader отображает VGA-буфер один к одному, через это работает WRITER
    assert_eq!(translate(0xb8000), Some(PhysAddr::new(0xb8000)));
}

#[test_case]
fn test_translate_keeps_page_offset() {
    // смещение внутри страницы (младшие 12 бит) не меняется при трансляции
    assert_eq!(translate(0xb8123), Some(PhysAddr::new(0xb8123)));
}

#[test_case]
fn test_translate_kernel_code() {
    // код ядра выполняется, значит его страницы точно отображены
    let addr = test_kernel_main as *const () as u64;
    assert!(translate(addr).is_some());
}

#[test_case]
fn test_translate_unmapped_returns_none() {
    // нулевая страница не отображена, чтобы разыменование null давало page fault
    assert_eq!(translate(0), None);
}

#[test_case]
fn test_translate_matches_offset_page_table() {
    // ручная трансляция должна совпадать с реализацией из крейта x86_64
    let paging = PAGING.lock();
    let (mapper, _) = paging.as_ref().unwrap();
    let addresses = [
        0xb8000,
        test_kernel_main as *const () as u64,
        0,
    ];
    for &addr in &addresses {
        let virt = VirtAddr::new(addr);
        assert_eq!(translate(addr), mapper.translate_addr(virt));
    }
}

#[test_case]
fn test_physical_memory_offset_mapping() {
    // physical_memory_offset + X отображается на физический адрес X.
    // Bootloader отображает физическую память страницами по 2 MiB (huge pages),
    // а ручная memory::translate_addr их не поддерживает и паникует,
    // поэтому здесь трансляцию делает OffsetPageTable
    let paging = PAGING.lock();
    let (mapper, _) = paging.as_ref().unwrap();
    let offset = boot_info().physical_memory_offset;
    for &phys in &[0, 0x1234, 0xb8000, 0x20_0000] {
        let virt = VirtAddr::new(offset + phys);
        assert_eq!(mapper.translate_addr(virt), Some(PhysAddr::new(phys)));
    }
}

#[test_case]
fn test_frame_allocator_gives_usable_aligned_distinct_frames() {
    let mut paging = PAGING.lock();
    let (_, frame_allocator) = paging.as_mut().unwrap();

    let mut frames: [Option<PhysFrame<Size4KiB>>; 8] = [None; 8];
    for slot in frames.iter_mut() {
        *slot = frame_allocator.allocate_frame();
    }

    for (i, frame) in frames.iter().enumerate() {
        let frame = frame.expect("в QEMU свободной памяти больше 8 фреймов");
        let addr = frame.start_address().as_u64();
        assert_eq!(addr % 4096, 0);

        let in_usable_region = boot_info().memory_map.iter().any(|r| {
            r.region_type == MemoryRegionType::Usable
                && r.range.start_addr() <= addr
                && addr < r.range.end_addr()
        });
        assert!(in_usable_region, "фрейм {:#x} вне Usable-региона", addr);

        // один фрейм не выдаётся дважды
        assert!(!frames[..i].contains(&Some(frame)));
    }
}

#[test_case]
fn test_create_example_mapping() {
    // адрес из статьи: для него ещё нет таблиц 3-го..1-го уровня, поэтому
    // map_to придётся взять фреймы у аллокатора
    let page = Page::containing_address(VirtAddr::new(0xdead_beaf_000));
    {
        let mut paging = PAGING.lock();
        let (mapper, frame_allocator) = paging.as_mut().unwrap();
        memory::create_example_mapping(page, mapper, frame_allocator);
    }

    let vga = PhysAddr::new(0xb8000);
    assert_eq!(translate(page.start_address().as_u64()), Some(vga));

    // запись через новую страницу видна по физическому адресу VGA-буфера
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    let vga_ptr: *const u64 = (phys_mem_offset() + vga.as_u64()).as_ptr();
    unsafe {
        page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e);
        assert_eq!(vga_ptr.offset(400).read_volatile(), 0x_f021_f077_f065_f04e);
    }
}
