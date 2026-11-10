use super::Key;

const ESC: u8 = 0x1b;

/// В каком месте последовательности байтов находится декодер
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Ждём новую клавишу
    Ground,
    /// Пришёл ESC, ждём `[` или `O`
    Escape,
    /// Внутри `ESC [ ...`: копим числовой параметр до финального байта
    Csi { param: u8 },
    /// После `ESC O`: следующий байт - клавиша
    Ss3,
    /// Внутри многобайтового символа UTF-8
    Utf8 {
        buf: [u8; 4],
        len: usize,
        need: usize,
    },
}

/// Превращает байты от терминала в клавиши.
///
/// Обычные символы терминал присылает как есть (не-ASCII - в UTF-8), а
/// стрелки и подобные клавиши - escape-последовательностями: стрелка вверх
/// это три байта `ESC [ A`
pub struct SerialDecoder {
    state: State,
    // терминал может закончить строку парой "\r\n", это одно нажатие Enter
    last_was_cr: bool,
}

impl SerialDecoder {
    pub fn new() -> Self {
        SerialDecoder {
            state: State::Ground,
            last_was_cr: false,
        }
    }

    /// Обрабатывает очередной байт. Клавиша получается не из каждого:
    /// внутри escape-последовательности или символа UTF-8 будет `None`
    pub fn feed(&mut self, byte: u8) -> Option<Key> {
        let after_cr = self.last_was_cr;
        self.last_was_cr = byte == b'\r';

        match self.state {
            State::Ground => self.ground(byte, after_cr),
            State::Escape => {
                self.state = match byte {
                    b'[' => State::Csi { param: 0 },
                    b'O' => State::Ss3,
                    // Alt + клавиша или одиночный Esc: не поддерживаем
                    // TODO: Alt + буква как Key::Alt(char); одиночный Esc отличать по таймауту
                    _ => State::Ground,
                };
                None
            }
            State::Csi { param } => match byte {
                b'0'..=b'9' => {
                    let digit = byte - b'0';
                    let param = param.saturating_mul(10).saturating_add(digit);
                    self.state = State::Csi { param };
                    None
                }
                // ESC [ 1 ; 5 C - стрелка с модификатором: модификатор отбрасываем
                // TODO: сохранять модификатор, чтобы Ctrl+←/→ переходили по словам
                b';' => {
                    self.state = State::Csi { param: 0 };
                    None
                }
                // финальный байт последовательности
                0x40..=0x7e => {
                    self.state = State::Ground;
                    match byte {
                        // TODO: F5-F12 - ESC [ 15 ~ ... ESC [ 24 ~
                        b'~' => match param {
                            1 | 7 => Some(Key::Home),
                            3 => Some(Key::Delete),
                            4 | 8 => Some(Key::End),
                            _ => None,
                        },
                        _ => cursor_key(byte),
                    }
                }
                // неожиданный байт: последовательность испорчена
                _ => {
                    self.state = State::Ground;
                    None
                }
            },
            // TODO: F1-F4 - ESC O P ... ESC O S
            State::Ss3 => {
                self.state = State::Ground;
                cursor_key(byte)
            }
            State::Utf8 { mut buf, len, need } => {
                if byte & 0b1100_0000 != 0b1000_0000 {
                    // ждали продолжение символа, а пришло начало нового:
                    // испорченный символ выбрасываем, а байт разбираем заново
                    self.state = State::Ground;
                    return self.ground(byte, after_cr);
                }
                buf[len] = byte;
                let len = len + 1;
                if len < need {
                    self.state = State::Utf8 { buf, len, need };
                    return None;
                }
                self.state = State::Ground;
                let s = core::str::from_utf8(&buf[..len]).ok()?;
                s.chars().next().map(Key::Char)
            }
        }
    }

    fn ground(&mut self, byte: u8, after_cr: bool) -> Option<Key> {
        match byte {
            ESC => {
                self.state = State::Escape;
                None
            }
            b'\n' if after_cr => None,
            0x20..=0x7e => Some(Key::Char(byte as char)),
            0x00..=0x1f | 0x7f => Key::from_control(byte as char),
            // первый байт многобайтового символа: по старшим битам видно,
            // сколько всего в нём байтов
            0xc2..=0xdf => self.start_utf8(byte, 2),
            0xe0..=0xef => self.start_utf8(byte, 3),
            0xf0..=0xf4 => self.start_utf8(byte, 4),
            // байт продолжения без начала или недопустимый в UTF-8 байт
            _ => None,
        }
    }

    fn start_utf8(&mut self, byte: u8, need: usize) -> Option<Key> {
        let mut buf = [0; 4];
        buf[0] = byte;
        self.state = State::Utf8 { buf, len: 1, need };
        None
    }
}

/// Финальный байт `ESC [ X` или `ESC O X` для стрелок, Home и End
fn cursor_key(byte: u8) -> Option<Key> {
    match byte {
        b'A' => Some(Key::Up),
        b'B' => Some(Key::Down),
        b'C' => Some(Key::Right),
        b'D' => Some(Key::Left),
        b'H' => Some(Key::Home),
        b'F' => Some(Key::End),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
