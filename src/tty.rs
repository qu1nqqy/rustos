// Терминал: ввод с клавиатуры и из последовательного порта

mod key;
mod keyboard;
mod serial;

pub use key::Key;
pub use keyboard::KeyboardDecoder;
pub use serial::SerialDecoder;

use crate::drivers;
use futures_util::future;
use futures_util::stream::{self, Stream, StreamExt};

/// Поток нажатых клавиш сразу с двух источников: клавиатуры PS/2 и терминала
/// хоста через COM1. Вызывать можно только один раз
pub fn keys() -> impl Stream<Item = Key> + Unpin {
    let mut keyboard = KeyboardDecoder::new();
    let from_keyboard = drivers::keyboard::scancodes()
        .filter_map(move |scancode| future::ready(keyboard.feed(scancode)));

    let mut serial = SerialDecoder::new();
    let from_serial =
        drivers::serial::input().filter_map(move |byte| future::ready(serial.feed(byte)));

    // select опрашивает оба потока и отдаёт элементы того, где они есть
    stream::select(from_keyboard, from_serial)
}
