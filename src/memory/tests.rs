use super::*;
use bootloader::bootinfo::{FrameRange, MemoryRegion};
use lazy_static::lazy_static;

// Аллокатор проверяется на выдуманной карте памяти: он только читает карту,
// поэтому состояние CPU и настоящие таблицы страниц не затрагиваются.
// Работа с настоящими таблицами страниц проверяется в tests/paging.rs

fn region(start: u64, end: u64, region_type: MemoryRegionType) -> MemoryRegion {
    MemoryRegion {
        range: FrameRange::new(start, end),
        region_type,
    }
}

lazy_static! {
    // init() требует &'static MemoryMap, поэтому карта лежит в static
    static ref TEST_MEMORY_MAP: MemoryMap = {
        let mut map = MemoryMap::new();
        map.add_region(region(0x0000, 0x1000, MemoryRegionType::FrameZero));
        map.add_region(region(0x1000, 0x3000, MemoryRegionType::Usable));
        map.add_region(region(0x3000, 0x5000, MemoryRegionType::Kernel));
        map.add_region(region(0x10000, 0x11000, MemoryRegionType::Usable));
        map
    };
}

fn test_allocator() -> BootInfoFrameAllocator {
    unsafe { BootInfoFrameAllocator::init(&TEST_MEMORY_MAP) }
}

fn frame(addr: u64) -> PhysFrame {
    PhysFrame::containing_address(PhysAddr::new(addr))
}

#[test_case]
fn test_usable_frames_only_from_usable_regions() {
    // FrameZero и Kernel пропускаются, Usable-регионы режутся по 4 KiB
    let allocator = test_allocator();
    let mut frames = allocator.usable_frames();
    assert_eq!(frames.next(), Some(frame(0x1000)));
    assert_eq!(frames.next(), Some(frame(0x2000)));
    assert_eq!(frames.next(), Some(frame(0x10000)));
    assert_eq!(frames.next(), None);
}

#[test_case]
fn test_allocate_frame_returns_frames_in_order() {
    let mut allocator = test_allocator();
    assert_eq!(allocator.allocate_frame(), Some(frame(0x1000)));
    assert_eq!(allocator.allocate_frame(), Some(frame(0x2000)));
    assert_eq!(allocator.allocate_frame(), Some(frame(0x10000)));
}

#[test_case]
fn test_allocate_frame_none_when_exhausted() {
    let mut allocator = test_allocator();
    for _ in 0..3 {
        assert!(allocator.allocate_frame().is_some());
    }
    assert_eq!(allocator.allocate_frame(), None);
    // и повторный вызов тоже не выдаёт фрейм
    assert_eq!(allocator.allocate_frame(), None);
}

#[test_case]
fn test_allocators_are_independent() {
    // у каждого аллокатора свой счётчик next
    let mut first = test_allocator();
    first.allocate_frame();
    let mut second = test_allocator();
    assert_eq!(second.allocate_frame(), Some(frame(0x1000)));
}
