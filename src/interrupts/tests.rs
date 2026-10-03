use super::*;
use x86_64::VirtAddr;

// IDT здесь только строится (lazy_static), но не загружается в CPU:
// загрузка и вызов исключений проверяются в tests/

#[test_case]
fn test_breakpoint_handler_registered() {
    let expected = VirtAddr::from_ptr(breakpoint_handler as *const ());
    assert_eq!(IDT.breakpoint.handler_addr(), expected);
}

#[test_case]
fn test_double_fault_handler_registered() {
    let expected = VirtAddr::from_ptr(double_fault_handler as *const ());
    assert_eq!(IDT.double_fault.handler_addr(), expected);
}

#[test_case]
fn test_unhandled_exceptions_have_no_handler() {
    // обработчики пока есть только для breakpoint и double fault,
    // остальные исключения доходят до double fault
    assert_eq!(IDT.divide_error.handler_addr(), VirtAddr::zero());
    assert_eq!(IDT.page_fault.handler_addr(), VirtAddr::zero());
    assert_eq!(
        IDT.general_protection_fault.handler_addr(),
        VirtAddr::zero()
    );
}
