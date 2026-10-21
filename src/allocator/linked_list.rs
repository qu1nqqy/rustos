use super::{Locked, align_up};
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};

/// Свободный регион. Узел хранится в начале самого региона,
/// поэтому для списка не нужна отдельная память
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// Аллокатор на связном списке свободных регионов (free list).
/// Соседние свободные регионы не объединяются, поэтому со временем
/// куча фрагментируется
pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Создаёт пустой аллокатор
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// unsafe: вызывающий гарантирует, что память [heap_start, heap_start + heap_size)
    /// не используется. Вызывать один раз
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_region(heap_start, heap_size);
        }
    }

    /// Добавляет регион в начало списка
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // в свободном регионе должен помещаться ListNode
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        unsafe {
            node_ptr.write(node);
            self.head.next = Some(&mut *node_ptr)
        }
    }

    /// Ищет свободный регион заданного размера и выравнивания и удаляет
    /// его из списка. Возвращает узел региона и адрес начала аллокации
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // ссылка на текущий узел, начиная с фиктивного head
        let mut current = &mut self.head;
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // регион подходит: удаляем его из списка
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // не подходит: переходим к следующему
                current = current.next.as_mut().unwrap();
            }
        }

        // подходящих регионов нет
        None
    }

    /// Пробует разместить аллокацию в регионе. Возвращает адрес начала аллокации
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // регион слишком маленький
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // остаток слишком мал, чтобы хранить в нём ListNode
            // (нужно, потому что аллокация делит регион на занятую и свободную части)
            return Err(());
        }

        // регион подходит
        Ok(alloc_start)
    }

    /// Увеличивает размер и выравнивание так, чтобы освобождённый блок
    /// смог хранить ListNode. Возвращает (size, align)
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // подгоняем layout под ListNode
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                unsafe {
                    allocator.add_free_region(alloc_end, excess_size);
                }
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // подгоняем layout под ListNode
        let (size, _) = LinkedListAllocator::size_align(layout);

        unsafe { self.lock().add_free_region(ptr as usize, size) }
    }
}

#[cfg(test)]
mod tests;
