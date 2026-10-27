use super::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::{
    mem,
    ptr::{self, NonNull},
};

/// Размеры блоков.
///
/// Каждый размер должен быть степенью двойки, потому что он же используется
/// как выравнивание блока (а выравнивание всегда степень двойки)
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// Свободный блок. Размер хранить не нужно: он задаётся списком, в котором блок лежит
struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// Аллокатор блоков фиксированного размера: у каждого размера из BLOCK_SIZES
/// свой список свободных блоков. Аллокации больше 2048 байт и новые блоки,
/// когда список пуст, берутся у fallback-аллокатора
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Создаёт пустой аллокатор
    pub const fn new() -> Self {
        // [None; N] требует Copy, а &mut не Copy; константа копируется в каждый элемент
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// unsafe: вызывающий гарантирует, что память [heap_start, heap_start + heap_size)
    /// не используется. Вызывать один раз
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator
                .init(heap_start as *mut u8, heap_size);
        }
    }

    /// Выделяет память через fallback-аллокатор
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Выбирает подходящий размер блока для layout.
///
/// Возвращает индекс в массиве BLOCK_SIZES или None, если блок слишком большой
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // в списке нет блоков: берём новый у fallback-аллокатора
                        let block_size = BLOCK_SIZES[index];
                        // работает, только если все размеры блоков - степени двойки
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();
                        allocator.fallback_alloc(layout)
                    }
                }
            }
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                // проверяем, что в блоке хватает места для ListNode
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                }
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    allocator.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
