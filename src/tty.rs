// Терминал: ввод с клавиатуры и из последовательного порта, редактирование строки

mod key;
mod keyboard;
mod line;
mod serial;

pub use key::Key;
pub use keyboard::KeyboardDecoder;
pub use line::{Event, LineEditor};
pub use serial::SerialDecoder;

use crate::console::Console;
use crate::drivers::{self, vga};
use crate::print;
use alloc::boxed::Box;
use alloc::string::String;
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

/// Чем закончился `read_line`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadLine {
    Line(String),
    /// Ctrl+C
    Interrupted,
    /// Ctrl+D на пустой строке или поток клавиш закончился
    Eof,
}

/// Терминал: читает строки с редактированием и историей
pub struct Tty {
    keys: Box<dyn Stream<Item = Key> + Unpin + Send>,
    editor: LineEditor,
}

impl Tty {
    /// Терминал на клавиатуре и COM1. Создать можно только один: `keys()`
    /// вызывается один раз
    pub fn new() -> Self {
        Tty::with_keys(Box::new(keys()))
    }

    /// Терминал на произвольном потоке клавиш, например заранее заданных в тесте
    pub fn with_keys(keys: Box<dyn Stream<Item = Key> + Unpin + Send>) -> Self {
        Tty {
            keys,
            editor: LineEditor::new(0),
        }
    }

    pub fn history(&self) -> &[String] {
        self.editor.history()
    }

    /// Выводит `prompt` и читает строку, пока не нажат Enter
    pub async fn read_line(&mut self, prompt: &str) -> ReadLine {
        // строка вместе с приглашением должна поместиться в одну строку экрана:
        // перерисовка возвращает курсор только в начало текущей строки.
        // Последняя колонка остаётся под курсор
        let width = vga::BUFFER_WIDTH - 1;
        self.editor
            .set_max_len(width.saturating_sub(prompt.chars().count()));

        self.redraw(prompt);
        while let Some(key) = self.keys.next().await {
            match self.editor.handle(key) {
                Event::Continue => self.redraw(prompt),
                Event::Submit(line) => {
                    print!("\n");
                    return ReadLine::Line(line);
                }
                Event::Interrupt => {
                    self.redraw(prompt);
                    print!("^C\n");
                    self.editor.clear();
                    return ReadLine::Interrupted;
                }
                Event::Eof => {
                    print!("\n");
                    return ReadLine::Eof;
                }
                Event::ClearScreen => {
                    // очистить экран и перейти в левый верхний угол
                    print!("\x1b[2J\x1b[H");
                    self.redraw(prompt);
                }
            }
        }
        ReadLine::Eof
    }

    fn redraw(&self, prompt: &str) {
        self.editor
            .render(prompt, &mut Console)
            .expect("console write failed");
    }
}

#[cfg(test)]
mod tests;
