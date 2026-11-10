use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::VolatileRef;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// Память VGA-буфера читается только видеокартой, поэтому компилятор может
// решить, что запись в неё ни на что не влияет, и выбросить её.
// Volatile-доступ запрещает такую оптимизацию
impl Buffer {
    fn read(&self, row: usize, col: usize) -> ScreenChar {
        VolatileRef::from_ref(&self.chars[row][col]).as_ptr().read()
    }

    fn write(&mut self, row: usize, col: usize, screen_char: ScreenChar) {
        VolatileRef::from_mut_ref(&mut self.chars[row][col])
            .as_mut_ptr()
            .write(screen_char);
    }
}

/// Где Writer находится внутри escape-последовательности
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Escape {
    /// Обычный текст
    Ground,
    /// Пришёл ESC, ждём `[`
    Esc,
    /// Внутри `ESC [ ...`: копим числовой параметр до финального символа
    Csi { param: u16 },
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
    // последовательность может прийти по частям, например из разных
    // вызовов write_str при форматировании, поэтому состояние хранится в Writer
    escape: Escape,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.write(
                    row,
                    col,
                    ScreenChar {
                        ascii_character: byte,
                        color_code,
                    },
                );
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
        self.update_cursor();
    }

    fn write_char(&mut self, c: char) {
        match self.escape {
            Escape::Ground => match c {
                '\x1b' => self.escape = Escape::Esc,
                '\n' => self.new_line(),
                '\r' => self.column_position = 0,
                // входит в печатный ASCII диапазон
                ' '..='~' => self.write_byte(c as u8),
                // остального в кодировке VGA нет, выводим ■
                _ => self.write_byte(0xfe),
            },
            Escape::Esc => {
                self.escape = match c {
                    '[' => Escape::Csi { param: 0 },
                    // другие последовательности не поддерживаем
                    _ => Escape::Ground,
                }
            }
            Escape::Csi { param } => match c {
                '0'..='9' => {
                    let digit = c as u16 - '0' as u16;
                    let param = param.saturating_mul(10).saturating_add(digit);
                    self.escape = Escape::Csi { param };
                }
                // несколько параметров, например цвет ESC [ 1 ; 31 m:
                // запоминаем только последний
                ';' => self.escape = Escape::Csi { param: 0 },
                // финальный символ последовательности
                '@'..='~' => {
                    self.escape = Escape::Ground;
                    self.control_sequence(c, param);
                }
                // неожиданный символ: последовательность испорчена
                _ => self.escape = Escape::Ground,
            },
        }
    }

    /// Выполняет `ESC [ param command`. Курсор всегда в последней строке,
    /// поэтому команды двигают его только по горизонтали
    // TODO: `m` - цвета (ESC [ 31 m - красный, ESC [ 0 m - сброс), тогда приглашение
    // оболочки можно сделать зелёным на обоих устройствах
    // TODO (сложное): ESC [ n A - курсор вверх, чтобы ввод мог быть длиннее строки экрана
    fn control_sequence(&mut self, command: char, param: u16) {
        // у команд перемещения параметр 0 и пропущенный параметр означают 1
        let count = (param as usize).max(1);
        match command {
            // курсор влево
            'D' => self.column_position = self.column_position.saturating_sub(count),
            // курсор вправо
            'C' => {
                self.column_position = (self.column_position + count).min(BUFFER_WIDTH - 1);
            }
            // стереть от курсора до конца строки
            'K' if param == 0 => {
                for col in self.column_position..BUFFER_WIDTH {
                    self.buffer.write(BUFFER_HEIGHT - 1, col, self.blank());
                }
            }
            // очистить экран
            'J' if param == 2 => {
                for row in 0..BUFFER_HEIGHT {
                    self.clear_row(row);
                }
            }
            // курсор в начало (строка всегда последняя)
            'H' => self.column_position = 0,
            // остальное, например цвета (m), пропускаем
            _ => {}
        }
    }

    /// Переносит мигающий курсор видеокарты туда, куда будет записан
    /// следующий символ
    fn update_cursor(&self) {
        use x86_64::instructions::port::Port;

        let col = self.column_position.min(BUFFER_WIDTH - 1);
        let position = ((BUFFER_HEIGHT - 1) * BUFFER_WIDTH + col) as u16;
        // у контроллера CRT два порта: в 0x3D4 пишется номер регистра,
        // в 0x3D5 - его значение. Регистры 0x0F и 0x0E - младший и старший
        // байт позиции курсора
        let mut index = Port::<u8>::new(0x3d4);
        let mut data = Port::<u8>::new(0x3d5);
        unsafe {
            index.write(0x0f);
            data.write(position as u8);
            index.write(0x0e);
            data.write((position >> 8) as u8);
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.read(row, col);
                self.buffer.write(row - 1, col, character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        for col in 0..BUFFER_WIDTH {
            self.buffer.write(row, col, self.blank());
        }
    }

    fn blank(&self) -> ScreenChar {
        ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::LightGreen, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        escape: Escape::Ground,
    });
}

#[cfg(test)]
mod tests;
