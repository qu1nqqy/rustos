use crate::task::queue::{AsyncQueue, QueueStream};
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::spec::registers::IER;
use uart_16550::{Config, Uart16550Tty, backend::PioBackend};

lazy_static! {
    pub static ref SERIAL1: Mutex<Uart16550Tty<PioBackend>> = Mutex::new(unsafe {
        let config = Config {
            // прерывание, когда пришли данные
            interrupts: IER::DATA_READY,
            ..Config::default()
        };
        Uart16550Tty::new_port(0x3F8, config).expect("failed to initialize UART")
    });
}

static INPUT: AsyncQueue<u8> = AsyncQueue::new();

/// Настраивает COM1. Вызывать до включения прерываний
pub fn init() {
    // SERIAL1 создаётся при первом обращении. Без init это случилось бы при
    // первом выводе, и до него байты из терминала не вызывали бы прерываний
    lazy_static::initialize(&SERIAL1);
}

/// Вызывается обработчиком прерывания COM1: забирает все принятые байты.
///
/// Без читателя или при полной очереди байты выбрасываются: печатать
/// предупреждение здесь нельзя, оно само ушло бы в этот же порт
pub(crate) fn handle_interrupt() {
    let mut serial = SERIAL1.lock();
    while let Ok(byte) = serial.inner_mut().try_receive_byte() {
        let _ = INPUT.push(byte);
    }
}

/// Асинхронный поток байтов, принятых через COM1. Вызывать можно только один раз
pub fn input() -> QueueStream<u8> {
    INPUT.stream(256)
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("failed to write to serial");
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[cfg(test)]
mod tests;
