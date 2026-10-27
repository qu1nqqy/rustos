use super::*;
use x86_64::PrivilegeLevel;

// GDT и TSS здесь только строятся (lazy_static), но не загружаются в CPU:
// загрузка проверяется в tests/interrupts.rs и tests/stack_overflow.rs

#[test_case]
fn test_double_fault_ist_stack_set() {
    let ist = TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize];
    assert_ne!(ist, VirtAddr::zero());
}

#[test_case]
fn test_other_ist_entries_empty() {
    // TSS - packed-структура, на её поля нельзя брать ссылки, поэтому копируем
    let ist = TSS.interrupt_stack_table;
    for (i, &addr) in ist.iter().enumerate() {
        if i != DOUBLE_FAULT_IST_INDEX as usize {
            assert_eq!(addr, VirtAddr::zero(), "IST[{}]", i);
        }
    }
}

#[test_case]
fn test_gdt_layout() {
    // 0 - нулевой дескриптор, 1 - код ядра, 2..3 - TSS (занимает две записи)
    assert_eq!(GDT.0.entries().len(), 4);
    assert_eq!(GDT.1.code_selector.index(), 1);
    assert_eq!(GDT.1.tss_selector.index(), 2);
}

#[test_case]
fn test_selectors_are_ring0() {
    assert_eq!(GDT.1.code_selector.rpl(), PrivilegeLevel::Ring0);
    assert_eq!(GDT.1.tss_selector.rpl(), PrivilegeLevel::Ring0);
}
