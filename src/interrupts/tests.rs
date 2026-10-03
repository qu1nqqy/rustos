use super::*;
use x86_64::VirtAddr;

// IDT здесь только строится (lazy_static), но не загружается в CPU:
// загрузка и вызов int3 проверяются в tests/interrupts.rs

#[test_case]
fn test_breakpoint_handler_registered() {
    let expected = VirtAddr::from_ptr(breakpoint_handler as *const ());
    assert_eq!(IDT.breakpoint.handler_addr(), expected);
}

#[test_case]
fn test_unhandled_exceptions_have_no_handler() {
    // обработчики пока есть только для breakpoint, у остальных адрес нулевой
    assert_eq!(IDT.divide_error.handler_addr(), VirtAddr::zero());
    assert_eq!(IDT.page_fault.handler_addr(), VirtAddr::zero());
    assert_eq!(IDT.general_protection_fault.handler_addr(), VirtAddr::zero());
}
