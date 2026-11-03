use crate::println;
use crate::task::queue::{AsyncQueue, PushError, QueueStream};

static SCANCODES: AsyncQueue<u8> = AsyncQueue::new();

/// Вызывается обработчиком прерывания клавиатуры.
///
/// Не должна блокироваться и выделять память
pub(crate) fn add_scancode(scancode: u8) {
    match SCANCODES.push(scancode) {
        Ok(()) => {}
        Err(PushError::Full) => println!("WARNING: scancode queue full; dropping keyboard input"),
        Err(PushError::Uninit) => println!("WARNING: scancode queue uninitialized"),
    }
}

/// Асинхронный поток скан-кодов клавиатуры. Вызывать можно только один раз
pub fn scancodes() -> QueueStream<u8> {
    SCANCODES.stream(100)
}
