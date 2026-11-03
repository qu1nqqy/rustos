/// Нажатая клавиша, независимо от того, откуда она пришла:
/// с клавиатуры PS/2 или из терминала через последовательный порт
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Печатный символ
    Char(char),
    Enter,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    /// Ctrl + буква, буква всегда строчная: `Ctrl('c')`
    Ctrl(char),
}

impl Key {
    /// Разбирает управляющий ASCII-символ.
    ///
    /// И pc-keyboard, и терминал кодируют Ctrl + буква одинаково: Ctrl+A = 1,
    /// Ctrl+B = 2, ... Ctrl+Z = 26. Часть этих кодов исторически заняли
    /// другие клавиши: Tab (Ctrl+I), Enter (Ctrl+J, Ctrl+M), Backspace (Ctrl+H)
    pub fn from_control(c: char) -> Option<Key> {
        match c {
            '\n' | '\r' => Some(Key::Enter),
            '\u{8}' | '\u{7f}' => Some(Key::Backspace),
            '\t' => Some(Key::Tab),
            '\u{1}'..='\u{1a}' => {
                let letter = (b'a' + c as u8 - 1) as char;
                Some(Key::Ctrl(letter))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
