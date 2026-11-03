// Общий код для интеграционных тестов.
// tests/common/mod.rs не считается отдельным тестом, его подключают через `mod common;`

use bootloader::BootInfo;
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use rustos::mm::{allocator, paging};
use x86_64::VirtAddr;
use x86_64::instructions::interrupts;
use x86_64::instructions::port::Port;

/// Проверяет, что сообщение паники начинается с `prefix`, не выделяя память:
/// сообщение форматируется кусками, и каждый кусок сравнивается с ещё не
/// проверенной частью префикса
#[allow(dead_code)]
pub fn panic_message_starts_with(info: &PanicInfo, prefix: &str) -> bool {
    struct PrefixMatcher<'a> {
        rest: &'a [u8],
        ok: bool,
    }

    impl Write for PrefixMatcher<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let n = s.len().min(self.rest.len());
            if s.as_bytes()[..n] != self.rest[..n] {
                self.ok = false;
            }
            self.rest = &self.rest[n..];
            Ok(())
        }
    }

    let mut matcher = PrefixMatcher {
        rest: prefix.as_bytes(),
        ok: true,
    };
    let _ = write!(matcher, "{}", info.message());
    matcher.ok && matcher.rest.is_empty()
}

/// Инициализирует кучу так же, как kernel_main в main.rs.
/// Вызывать один раз: `paging::init` нельзя вызывать повторно
#[allow(dead_code)] // нужна не всем тестам, которые подключают common
pub fn init_heap(boot_info: &'static BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { paging::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { paging::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
}

/// Future, который один раз возвращает Pending, сразу разбудив себя.
/// Так задача уступает executor'у очередь, как `yield` в генераторе Python
pub struct YieldNow {
    yielded: bool,
}

impl core::future::Future for YieldNow {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context,
    ) -> core::task::Poll<()> {
        if self.yielded {
            core::task::Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            core::task::Poll::Pending
        }
    }
}

#[allow(dead_code)]
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

const PS2_STATUS_OUTPUT_FULL: u8 = 1 << 0;
const PS2_STATUS_INPUT_FULL: u8 = 1 << 1;
const PS2_CMD_WRITE_KEYBOARD_OUTPUT: u8 = 0xd2;

/// Эмулирует нажатие клавиши: команда 0xD2 контроллера PS/2 кладёт байт в его
/// выходной буфер так, будто его прислала клавиатура, и поднимает IRQ1.
/// Возвращается, когда обработчик прерывания прочитал скан-код.
/// Нужны rustos::init() и включённые прерывания
#[allow(dead_code)]
pub fn press_key(scancode: u8) {
    let mut status = Port::<u8>::new(0x64);
    let mut command = Port::<u8>::new(0x64);
    let mut data = Port::<u8>::new(0x60);
    unsafe {
        while status.read() & PS2_STATUS_INPUT_FULL != 0 {}
        command.write(PS2_CMD_WRITE_KEYBOARD_OUTPUT);
        while status.read() & PS2_STATUS_INPUT_FULL != 0 {}
        data.write(scancode);

        // буфер освободится, когда обработчик клавиатуры прочитает порт 0x60
        while status.read() & PS2_STATUS_OUTPUT_FULL != 0 {
            x86_64::instructions::hlt();
        }
    }
}

const COM1: u16 = 0x3f8;
const MCR_LOOPBACK: u8 = 1 << 4;
const LSR_THR_EMPTY: u8 = 1 << 5;
const LSR_TRANSMITTER_EMPTY: u8 = 1 << 6;
/// Приёмный FIFO UART - 16 байт, больше за раз отправлять нельзя: пока
/// прерывания выключены, обработчик их не заберёт и лишние потеряются
pub const SERIAL_CHUNK: usize = 16;

/// Эмулирует байты, пришедшие из терминала в COM1. В режиме loopback UART
/// принимает собственные отправленные байты, и они вызывают то же прерывание
/// IRQ4, что и байты из терминала.
///
/// Прерывания выключены на всё время loopback: иначе вывод тестов через serial
/// тоже ушёл бы не в терминал, а в приёмник
#[allow(dead_code)]
pub fn send_from_terminal(bytes: &[u8]) {
    assert!(
        bytes.len() <= SERIAL_CHUNK,
        "at most {} bytes at once",
        SERIAL_CHUNK
    );
    let mut data = Port::<u8>::new(COM1);
    let mut mcr = Port::<u8>::new(COM1 + 4);
    let mut lsr = Port::<u8>::new(COM1 + 5);
    interrupts::without_interrupts(|| unsafe {
        let old_mcr = mcr.read();
        mcr.write(old_mcr | MCR_LOOPBACK);
        for &byte in bytes {
            while lsr.read() & LSR_THR_EMPTY == 0 {}
            data.write(byte);
        }
        while lsr.read() & LSR_TRANSMITTER_EMPTY == 0 {}
        mcr.write(old_mcr);
    });
}
