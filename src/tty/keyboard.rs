use super::Key;
use pc_keyboard::{DecodedKey, HandleControl, KeyCode, PS2Keyboard, ScancodeSet1, layouts};

/// Превращает скан-коды клавиатуры PS/2 в клавиши
pub struct KeyboardDecoder {
    keyboard: PS2Keyboard<layouts::Us104Key, ScancodeSet1>,
}

impl KeyboardDecoder {
    pub fn new() -> Self {
        KeyboardDecoder {
            keyboard: PS2Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                // Ctrl + буква превращается в управляющий символ, как в терминале
                HandleControl::MapLettersToUnicode,
            ),
        }
    }

    /// Обрабатывает очередной скан-код. Клавиша получается не из каждого:
    /// отпускание, Shift или первый байт двухбайтового кода дают `None`
    pub fn feed(&mut self, scancode: u8) -> Option<Key> {
        let event = self.keyboard.add_byte(scancode).ok()??;
        match self.keyboard.process_keyevent(event)? {
            // pc-keyboard отдаёт Delete как DEL (0x7f), а терминал так кодирует
            // Backspace, поэтому Delete разбирается до from_control
            DecodedKey::Unicode('\u{7f}') => Some(Key::Delete),
            DecodedKey::Unicode(c) if c.is_control() => Key::from_control(c),
            DecodedKey::Unicode(c) => Some(Key::Char(c)),
            DecodedKey::RawKey(code) => match code {
                KeyCode::ArrowLeft => Some(Key::Left),
                KeyCode::ArrowRight => Some(Key::Right),
                KeyCode::ArrowUp => Some(Key::Up),
                KeyCode::ArrowDown => Some(Key::Down),
                KeyCode::Home => Some(Key::Home),
                KeyCode::End => Some(Key::End),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests;
