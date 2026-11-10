use crate::arch::x86_64::without_interrupts;
use crate::drivers::{serial, vga};
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

// В тестах serial занят под отчёт тест-раннера, поэтому по умолчанию консоль
// пишет только на экран, а дублирование в serial включает kernel_main
static SERIAL_ENABLED: AtomicBool = AtomicBool::new(false);

/// Включает дублирование вывода консоли в последовательный порт
pub fn enable_serial() {
    SERIAL_ENABLED.store(true, Ordering::Relaxed);
}

/// Выключает дублирование вывода консоли в последовательный порт
pub fn disable_serial() {
    SERIAL_ENABLED.store(false, Ordering::Relaxed);
}

// TODO: disable_vga() по аналогии с serial: на сервере без экрана вывод нужен только в COM1

pub fn is_serial_enabled() -> bool {
    SERIAL_ENABLED.load(Ordering::Relaxed)
}

/// Консоль как приёмник `write!`: то же, что `print!`, но её можно передать
/// туда, где ждут `impl fmt::Write`
pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        _print(format_args!("{}", s));
        Ok(())
    }
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

/// Печатает в консоль: на экран и, если включено, в последовательный порт
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// Печатает в консоль, добавляя перевод строки
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[cfg(test)]
mod tests;
