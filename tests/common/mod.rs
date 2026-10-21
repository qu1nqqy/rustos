// Общий код для интеграционных тестов.
// tests/common/mod.rs не считается отдельным тестом, его подключают через `mod common;`

use bootloader::BootInfo;
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use rustos::{allocator, memory};
use x86_64::VirtAddr;

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
/// Вызывать один раз: `memory::init` нельзя вызывать повторно
#[allow(dead_code)] // нужна не всем тестам, которые подключают common
pub fn init_heap(boot_info: &'static BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
}
