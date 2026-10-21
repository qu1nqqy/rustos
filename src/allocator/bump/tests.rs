use super::*;

// Аллокатор работает с обычным массивом вместо настоящей кучи:
// он только записывает адреса, поэтому таблицы страниц не нужны.
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

fn test_allocator() -> Locked<BumpAllocator> {
    let allocator = Locked::new(BumpAllocator::new());
    unsafe { allocator.lock().init(heap_start(), TEST_HEAP_SIZE) };
    allocator
}

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

#[test_case]
fn test_init_sets_bounds() {
    let allocator = test_allocator();
    let bump = allocator.lock();
    assert_eq!(bump.heap_start, heap_start());
    assert_eq!(bump.heap_end, heap_start() + TEST_HEAP_SIZE);
    assert_eq!(bump.next, heap_start());
    assert_eq!(bump.allocations, 0);
}

#[test_case]
fn test_allocations_go_one_after_another() {
    let allocator = test_allocator();
    unsafe {
        assert_eq!(allocator.alloc(layout(8, 8)) as usize, heap_start());
        assert_eq!(allocator.alloc(layout(16, 8)) as usize, heap_start() + 8);
        assert_eq!(allocator.alloc(layout(8, 8)) as usize, heap_start() + 24);
    }
    assert_eq!(allocator.lock().allocations, 3);
}

#[test_case]
fn test_alloc_respects_alignment() {
    let allocator = test_allocator();
    unsafe {
        allocator.alloc(layout(1, 1));
        // next = start + 1, выравнивается до start + 16
        assert_eq!(allocator.alloc(layout(4, 16)) as usize, heap_start() + 16);
    }
}

#[test_case]
fn test_alloc_null_when_out_of_memory() {
    let allocator = test_allocator();
    unsafe {
        assert!(!allocator.alloc(layout(TEST_HEAP_SIZE, 1)).is_null());
        assert!(allocator.alloc(layout(1, 1)).is_null());
    }
    // неудачная аллокация не считается
    assert_eq!(allocator.lock().allocations, 1);
}

#[test_case]
fn test_alloc_larger_than_heap_fails() {
    let allocator = test_allocator();
    unsafe {
        assert!(allocator.alloc(layout(TEST_HEAP_SIZE + 1, 1)).is_null());
    }
}

#[test_case]
fn test_partial_dealloc_does_not_reuse_memory() {
    let allocator = test_allocator();
    unsafe {
        let first = allocator.alloc(layout(8, 8));
        allocator.alloc(layout(8, 8));
        allocator.dealloc(first, layout(8, 8));
        // пока жива вторая аллокация, next не сбрасывается
        assert_eq!(allocator.alloc(layout(8, 8)) as usize, heap_start() + 16);
    }
}

#[test_case]
fn test_dealloc_all_resets_heap() {
    let allocator = test_allocator();
    unsafe {
        let first = allocator.alloc(layout(8, 8));
        let second = allocator.alloc(layout(8, 8));
        allocator.dealloc(first, layout(8, 8));
        allocator.dealloc(second, layout(8, 8));
        assert_eq!(allocator.alloc(layout(8, 8)) as usize, heap_start());
    }
}
