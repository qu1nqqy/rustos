use super::*;

// Аллокатор работает с обычным массивом вместо настоящей кучи. Узлы списка
// записываются прямо в массив, таблицы страниц не меняются.
// Каждый тест создаёт новый аллокатор поверх того же массива

const TEST_HEAP_SIZE: usize = 256;

// поле не читается напрямую: тесты берут только адрес массива
#[allow(dead_code)]
#[repr(align(4096))]
struct TestHeap([u8; TEST_HEAP_SIZE]);

static mut TEST_HEAP: TestHeap = TestHeap([0; TEST_HEAP_SIZE]);

fn heap_start() -> usize {
    (&raw mut TEST_HEAP) as usize
}

fn allocator_with_size(size: usize) -> Locked<LinkedListAllocator> {
    let allocator = Locked::new(LinkedListAllocator::new());
    unsafe { allocator.lock().init(heap_start(), size) };
    allocator
}

fn test_allocator() -> Locked<LinkedListAllocator> {
    allocator_with_size(TEST_HEAP_SIZE)
}

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

/// Свободные регионы (адрес, размер) в порядке списка
fn free_regions(allocator: &Locked<LinkedListAllocator>) -> [(usize, usize); 8] {
    let mut regions = [(0, 0); 8];
    let list = allocator.lock();
    let mut current = &list.head.next;
    let mut i = 0;
    while let Some(region) = current {
        regions[i] = (region.start_addr(), region.size);
        current = &region.next;
        i += 1;
    }
    regions
}

#[test_case]
fn test_size_align_fits_list_node() {
    // ListNode: size + next = 16 байт, выравнивание 8
    assert_eq!(mem::size_of::<ListNode>(), 16);
    assert_eq!(LinkedListAllocator::size_align(layout(1, 1)), (16, 8));
    assert_eq!(LinkedListAllocator::size_align(layout(17, 1)), (24, 8));
    assert_eq!(LinkedListAllocator::size_align(layout(24, 8)), (24, 8));
    // большее выравнивание сохраняется
    assert_eq!(LinkedListAllocator::size_align(layout(8, 32)), (32, 32));
}

#[test_case]
fn test_init_adds_one_free_region() {
    let allocator = test_allocator();
    let regions = free_regions(&allocator);
    assert_eq!(regions[0], (heap_start(), TEST_HEAP_SIZE));
    assert_eq!(regions[1], (0, 0));
}

#[test_case]
fn test_alloc_splits_region() {
    let allocator = test_allocator();
    let ptr = unsafe { allocator.alloc(layout(32, 8)) };
    assert_eq!(ptr as usize, heap_start());
    // остаток региона снова в списке
    let regions = free_regions(&allocator);
    assert_eq!(regions[0], (heap_start() + 32, TEST_HEAP_SIZE - 32));
}

#[test_case]
fn test_dealloc_returns_region_to_list() {
    let allocator = test_allocator();
    unsafe {
        let ptr = allocator.alloc(layout(32, 8));
        allocator.dealloc(ptr, layout(32, 8));
    }
    // освобождённый регион добавляется в начало списка; соседние регионы не объединяются
    let regions = free_regions(&allocator);
    assert_eq!(regions[0], (heap_start(), 32));
    assert_eq!(regions[1], (heap_start() + 32, TEST_HEAP_SIZE - 32));
}

#[test_case]
fn test_freed_memory_reused() {
    let allocator = test_allocator();
    unsafe {
        let first = allocator.alloc(layout(32, 8));
        allocator.alloc(layout(32, 8));
        allocator.dealloc(first, layout(32, 8));
        assert_eq!(allocator.alloc(layout(32, 8)), first);
    }
}

#[test_case]
fn test_too_small_region_skipped() {
    let allocator = test_allocator();
    unsafe {
        let small = allocator.alloc(layout(16, 8));
        allocator.alloc(layout(16, 8));
        allocator.dealloc(small, layout(16, 8));
        // первый регион в списке (16 байт) мал, берётся следующий
        let big = allocator.alloc(layout(64, 8));
        assert_eq!(big as usize, heap_start() + 32);
    }
}

#[test_case]
fn test_excess_smaller_than_list_node_rejected() {
    // после аллокации 24 из 32 байт останется 8 байт: ListNode в них не
    // поместится, поэтому регион не подходит
    let allocator = allocator_with_size(32);
    unsafe {
        assert!(allocator.alloc(layout(24, 8)).is_null());
        // а ровно 32 или 16 (остаток 16) - подходит
        assert!(!allocator.alloc(layout(16, 8)).is_null());
        assert!(!allocator.alloc(layout(16, 8)).is_null());
    }
}

#[test_case]
fn test_alloc_respects_alignment() {
    let allocator = test_allocator();
    unsafe {
        allocator.alloc(layout(16, 8));
        let ptr = allocator.alloc(layout(64, 64));
        assert_eq!(ptr as usize % 64, 0);
    }
}

#[test_case]
fn test_alloc_null_when_out_of_memory() {
    let allocator = test_allocator();
    unsafe {
        assert!(!allocator.alloc(layout(TEST_HEAP_SIZE, 8)).is_null());
        assert!(allocator.alloc(layout(1, 1)).is_null());
    }
}
