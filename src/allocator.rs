use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use fixed_size_block::FixedSizeBlockAllocator;
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB, mapper::MapToError,
    },
};

pub mod bump;
pub mod fixed_size_block;
pub mod linked_list;

// Куча - отдельная область виртуальной памяти. Адрес выбран так, чтобы
// его было легко узнать в отладчике и он не пересекался с другими отображениями
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

/// Аллокатор, который всегда возвращает ошибку (null)
pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}

// Через #[global_allocator] работают Box, Vec и остальные типы из alloc.
// До init_heap куча пустая, и любая аллокация завершится ошибкой.
// Вместо FixedSizeBlockAllocator можно подставить
// Locked<bump::BumpAllocator> или Locked<linked_list::LinkedListAllocator>
#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

/// Отображает страницы кучи на свободные фреймы и инициализирует аллокатор
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // unsafe: страницы кучи только что отображены и больше ничем не используются,
    // а init вызывается один раз
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

/// Обёртка над spin::Mutex. Методы GlobalAlloc принимают &self, а мьютекс
/// даёт изменяемый доступ к аллокатору. Реализовать GlobalAlloc прямо для
/// spin::Mutex<A> нельзя: и трейт, и тип объявлены в других крейтах
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<'_, A> {
        self.inner.lock()
    }
}

/// Выравнивает `addr` вверх до `align`. `align` должен быть степенью двойки:
/// тогда `!(align - 1)` - маска, которая обнуляет младшие биты
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests;
