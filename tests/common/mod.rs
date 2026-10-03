// Общий код для интеграционных тестов с harness = false.
// tests/common/mod.rs не считается отдельным тестом, его подключают через `mod common;`

use core::fmt::{self, Write};
use core::panic::PanicInfo;

/// Проверяет, что сообщение паники начинается с `prefix`, не выделяя память:
/// сообщение форматируется кусками, и каждый кусок сравнивается с ещё не
/// проверенной частью префикса
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
