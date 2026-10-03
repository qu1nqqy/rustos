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
fn test_pic_offsets_after_cpu_exceptions() {
    // векторы 0..32 заняты исключениями CPU, по 8 линий IRQ на каждый PIC
    assert_eq!(PIC_1_OFFSET, 32);
    assert_eq!(PIC_2_OFFSET, PIC_1_OFFSET + 8);
}

#[test_case]
fn test_interrupt_index_matches_irq_lines() {
    // таймер - IRQ0, клавиатура - IRQ1 первого PIC
    assert_eq!(InterruptIndex::Timer.as_u8(), PIC_1_OFFSET);
    assert_eq!(InterruptIndex::Keyboard.as_u8(), PIC_1_OFFSET + 1);
}

#[test_case]
fn test_timer_handler_registered() {
    let expected = VirtAddr::from_ptr(timer_interrupt_handler as *const ());
    assert_eq!(IDT[InterruptIndex::Timer.as_u8()].handler_addr(), expected);
}

#[test_case]
fn test_keyboard_handler_registered() {
    let expected = VirtAddr::from_ptr(keyboard_interrupt_handler as *const ());
    assert_eq!(IDT[InterruptIndex::Keyboard.as_u8()].handler_addr(), expected);
}

#[test_case]
fn test_unhandled_exceptions_have_no_handler() {
    // из исключений CPU обработчики есть только у breakpoint и double fault,
    // остальные доходят до double fault
    assert_eq!(IDT.divide_error.handler_addr(), VirtAddr::zero());
    assert_eq!(IDT.page_fault.handler_addr(), VirtAddr::zero());
    assert_eq!(
        IDT.general_protection_fault.handler_addr(),
        VirtAddr::zero()
    );
}
