#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod common;

use alloc::{boxed::Box, format, rc::Rc, string::String, vec::Vec};
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use rustos::allocator::{HEAP_SIZE, HEAP_START};
use spin::Once;
use x86_64::structures::paging::{OffsetPageTable, PageTable, Translate};
use x86_64::{VirtAddr, registers::control::Cr3};

// Куча настраивается через таблицы страниц, поэтому тесты в отдельном образе.
// rustos::init() не нужен: аллокатору прерывания не нужны

entry_point!(test_kernel_main);

static BOOT_INFO: Once<&'static BootInfo> = Once::new();

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    BOOT_INFO.call_once(|| boot_info);
    common::init_heap(boot_info);

    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

fn in_heap(addr: usize) -> bool {
    (HEAP_START..HEAP_START + HEAP_SIZE).contains(&addr)
}

#[test_case]
fn test_simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

#[test_case]
fn test_box_lives_in_heap() {
    let heap_value = Box::new(42u64);
    assert!(in_heap(&*heap_value as *const u64 as usize));
}

#[test_case]
fn test_large_vec() {
    // при росте Vec память перевыделяется, а старые буферы освобождаются
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

#[test_case]
fn test_many_boxes() {
    // в сумме выделяется больше HEAP_SIZE, поэтому без переиспользования
    // освобождённой памяти тест не пройдёт
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn test_many_boxes_long_lived() {
    // одна аллокация живёт весь тест. Bump-аллокатор переиспользует память,
    // только когда освобождено всё, поэтому с ним этот тест упадёт
    let long_lived = Box::new(1);
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*long_lived, 1);
}

#[test_case]
fn test_mixed_sizes() {
    // блоки разных размеров (и больше 2048 байт - через fallback-аллокатор)
    // не должны перекрываться
    let small = Box::new([1u8; 8]);
    let medium = Box::new([2u8; 500]);
    let large = Box::new([3u8; 4000]);
    let small_2 = Box::new([4u8; 8]);
    assert!(small.iter().all(|&b| b == 1));
    assert!(medium.iter().all(|&b| b == 2));
    assert!(large.iter().all(|&b| b == 3));
    assert!(small_2.iter().all(|&b| b == 4));
}

#[test_case]
fn test_freed_memory_reused() {
    let first = Box::new([0u8; 1024]);
    let addr = first.as_ptr() as usize;
    drop(first);
    let second = Box::new([0u8; 1024]);
    assert_eq!(second.as_ptr() as usize, addr);
}

#[test_case]
fn test_rc_reference_count() {
    let reference_counted = Rc::new(Vec::from([1, 2, 3]));
    let cloned_reference = reference_counted.clone();
    assert_eq!(Rc::strong_count(&cloned_reference), 2);
    drop(reference_counted);
    assert_eq!(Rc::strong_count(&cloned_reference), 1);
    assert_eq!(*cloned_reference, [1, 2, 3]);
}

#[test_case]
fn test_string_format() {
    let mut s = String::from("heap");
    s.push_str(" works");
    assert_eq!(format!("{}: {}", s, 42), "heap works: 42");
}

#[test_case]
fn test_too_large_allocation_fails() {
    // try_reserve возвращает ошибку вместо паники, когда памяти не хватает
    let mut vec: Vec<u8> = Vec::new();
    assert!(vec.try_reserve(HEAP_SIZE + 1).is_err());
    // после неудачной аллокации куча продолжает работать
    let x = Box::new(1);
    assert_eq!(*x, 1);
}

#[test_case]
fn test_heap_pages_mapped() {
    // init_heap отображает ровно страницы [HEAP_START, HEAP_START + HEAP_SIZE)
    let offset = VirtAddr::new(BOOT_INFO.get().unwrap().physical_memory_offset);
    let (level_4_frame, _) = Cr3::read();
    let table = (offset + level_4_frame.start_address().as_u64()).as_mut_ptr::<PageTable>();
    let mapper = unsafe { OffsetPageTable::new(&mut *table, offset) };

    let translate = |addr: usize| mapper.translate_addr(VirtAddr::new(addr as u64));
    assert!(translate(HEAP_START).is_some());
    assert!(translate(HEAP_START + HEAP_SIZE - 1).is_some());
    assert!(translate(HEAP_START + HEAP_SIZE).is_none());
    assert!(translate(HEAP_START - 1).is_none());
}
