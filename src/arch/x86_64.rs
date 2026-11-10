pub mod gdt;
pub mod interrupts;
pub mod power;

pub use x86_64::instructions::interrupts::without_interrupts;

/// Загружает GDT, TSS и IDT, настраивает PIC и включает прерывания
pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe {
        let mut pics = interrupts::PICS.lock();
        pics.initialize();
        pics.write_masks(interrupts::PIC_1_MASK, interrupts::PIC_2_MASK);
    }
    x86_64::instructions::interrupts::enable();
}

/// Бесконечный цикл, в котором CPU спит до следующего прерывания
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
