use crate::arch::x86_64::without_interrupts;
use crate::drivers::{serial, vga};
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

static SERIAL_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable_serial() {
    SERIAL_ENABLED.store(true, Ordering::Relaxed);
}

pub fn disable_serial() {
    SERIAL_ENABLED.store(false, Ordering::Relaxed);
}

pub fn is_serial_enabled() -> bool {
    SERIAL_ENABLED.load(Ordering::Relaxed)
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // оба lock'а берёт и код из обработчиков прерываний (например, page fault
    // печатает через println!), поэтому прерывания выключены на всё время вывода
    without_interrupts(|| {
        vga::WRITER.lock().write_fmt(args).unwrap();
        if is_serial_enabled() {
            serial::SERIAL1.lock().write_fmt(args).unwrap();
        }
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[cfg(test)]
mod tests;
