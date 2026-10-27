pub mod gdt;
pub mod interrupts;

/// Загружает GDT, TSS и IDT, настраивает PIC и включает прерывания
pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

/// Бесконечный цикл, в котором CPU спит до следующего прерывания
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
