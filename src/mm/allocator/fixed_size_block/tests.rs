use super::*;

// Аллокатор работает с обычным массивом вместо настоящей кучи. Узлы списков
// записываются прямо в массив, таблицы страниц не меняются.
// Каждый тест создаёт новый аллокатор поверх того же массива

const TEST_HEAP_SIZE: usize = 16 * 1024;

// поле не читается напрямую: тесты берут только адрес массива
#[allow(dead_code)]
#[repr(align(4096))]
struct TestHeap([u8; TEST_HEAP_SIZE]);

static mut TEST_HEAP: TestHeap = TestHeap([0; TEST_HEAP_SIZE]);

fn heap_start() -> usize {
    (&raw mut TEST_HEAP) as usize
}

fn test_allocator() -> Locked<FixedSizeBlockAllocator> {
    let allocator = Locked::new(FixedSizeBlockAllocator::new());
    unsafe { allocator.lock().init(heap_start(), TEST_HEAP_SIZE) };
    allocator
}

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

fn in_heap(ptr: *mut u8) -> bool {
    (heap_start()..heap_start() + TEST_HEAP_SIZE).contains(&(ptr as usize))
}

#[test_case]
fn test_block_sizes_powers_of_two() {
    // размер блока используется как его выравнивание
    for &size in BLOCK_SIZES {
        assert!(size.is_power_of_two());
    }
    assert!(BLOCK_SIZES.windows(2).all(|w| w[0] < w[1]));
}

#[test_case]
fn test_smallest_block_fits_list_node() {
    assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[0]);
    assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[0]);
}

#[test_case]
fn test_list_index_picks_smallest_fitting_block() {
    assert_eq!(list_index(&layout(1, 1)), Some(0));
    assert_eq!(list_index(&layout(8, 8)), Some(0));
    assert_eq!(list_index(&layout(9, 1)), Some(1));
    assert_eq!(list_index(&layout(2048, 8)), Some(BLOCK_SIZES.len() - 1));
}

#[test_case]
fn test_list_index_uses_alignment() {
    // блок 8 байт с выравниванием 64 попадает в список блоков по 64
    assert_eq!(list_index(&layout(8, 64)), Some(3));
}

#[test_case]
fn test_list_index_none_for_large_allocations() {
    assert_eq!(list_index(&layout(2049, 1)), None);
    assert_eq!(list_index(&layout(8, 4096)), None);
}

#[test_case]
fn test_lists_empty_after_new() {
    let allocator = test_allocator();
    assert!(
        allocator
            .lock()
            .list_heads
            .iter()
            .all(|head| head.is_none())
    );
}

#[test_case]
fn test_block_aligned_to_its_size() {
    let allocator = test_allocator();
    unsafe {
        allocator.alloc(layout(8, 8));
        // 24 байта -> блок 32, выровненный на 32
        let ptr = allocator.alloc(layout(24, 8));
        assert!(in_heap(ptr));
        assert_eq!(ptr as usize % 32, 0);
    }
}

#[test_case]
fn test_dealloc_puts_block_into_list() {
    let allocator = test_allocator();
    unsafe {
        let ptr = allocator.alloc(layout(16, 8));
        allocator.dealloc(ptr, layout(16, 8));
    }
    let list = allocator.lock();
    let head = list.list_heads[1].as_ref().expect("блок не попал в список");
    assert!(head.next.is_none());
}

#[test_case]
fn test_freed_block_reused_lifo() {
    let allocator = test_allocator();
    unsafe {
        let first = allocator.alloc(layout(16, 8));
        let second = allocator.alloc(layout(16, 8));
        allocator.dealloc(first, layout(16, 8));
        allocator.dealloc(second, layout(16, 8));
        // список - стек: последним освобождён second, он и выдаётся первым
        assert_eq!(allocator.alloc(layout(16, 8)), second);
        assert_eq!(allocator.alloc(layout(16, 8)), first);
    }
}

#[test_case]
fn test_size_classes_do_not_share_blocks() {
    let allocator = test_allocator();
    unsafe {
        let small = allocator.alloc(layout(16, 8));
        allocator.dealloc(small, layout(16, 8));
        let bigger = allocator.alloc(layout(32, 8));
        assert_ne!(bigger, small);
    }
}

#[test_case]
fn test_large_allocation_uses_fallback() {
    let allocator = test_allocator();
    unsafe {
        let ptr = allocator.alloc(layout(4096, 8));
        assert!(in_heap(ptr));
        allocator.dealloc(ptr, layout(4096, 8));
        // большой блок возвращается fallback-аллокатору, а не в списки
        assert!(
            allocator
                .lock()
                .list_heads
                .iter()
                .all(|head| head.is_none())
        );
        assert_eq!(allocator.alloc(layout(4096, 8)), ptr);
    }
}

#[test_case]
fn test_alloc_null_when_out_of_memory() {
    let allocator = test_allocator();
    unsafe {
        assert!(allocator.alloc(layout(TEST_HEAP_SIZE + 1, 8)).is_null());
    }
}
