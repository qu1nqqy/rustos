use super::*;

// Здесь проверяются только константы и Dummy: глобальный аллокатор
// в unit-тестах не инициализирован (init_heap меняет таблицы страниц),
// поэтому Box и Vec тут использовать нельзя. Работа кучи проверяется
// в tests/heap_allocation.rs

#[test_case]
fn test_heap_start_page_aligned() {
    // map_to отображает целые страницы, иначе часть кучи окажется
    // на странице, которая не будет отображена
    assert_eq!(HEAP_START % 4096, 0);
}

#[test_case]
fn test_heap_size_whole_pages() {
    assert_eq!(HEAP_SIZE % 4096, 0);
    assert_eq!(HEAP_SIZE / 4096, 25);
}

#[test_case]
fn test_heap_start_canonical() {
    // адрес должен быть каноническим (биты 48..64 копируют бит 47),
    // иначе VirtAddr::new в init_heap запаникует
    assert!(VirtAddr::try_new(HEAP_START as u64).is_ok());
    assert!(VirtAddr::try_new((HEAP_START + HEAP_SIZE - 1) as u64).is_ok());
}

#[test_case]
fn test_dummy_alloc_returns_null() {
    let layout = Layout::from_size_align(8, 8).unwrap();
    let ptr = unsafe { Dummy.alloc(layout) };
    assert!(ptr.is_null());
}

#[test_case]
fn test_align_up_already_aligned() {
    assert_eq!(align_up(0, 8), 0);
    assert_eq!(align_up(0x1000, 4096), 0x1000);
}

#[test_case]
fn test_align_up_rounds_up() {
    assert_eq!(align_up(1, 8), 8);
    assert_eq!(align_up(9, 8), 16);
    assert_eq!(align_up(0x1001, 4096), 0x2000);
}

#[test_case]
fn test_align_up_align_one() {
    // выравнивание 1 ничего не меняет
    assert_eq!(align_up(13, 1), 13);
}

#[test_case]
fn test_locked_gives_mutable_access() {
    let locked = Locked::new(1);
    *locked.lock() += 1;
    assert_eq!(*locked.lock(), 2);
}
