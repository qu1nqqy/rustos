use super::Key;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Что сделать после нажатия клавиши
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Строка ещё не готова, её нужно перерисовать
    Continue,
    /// Нажат Enter: строка готова
    Submit(String),
    /// Ctrl+C: строку нужно отбросить. Курсор уже в конце строки, чтобы
    /// после неё можно было напечатать `^C`; очищает строку `clear`
    Interrupt,
    /// Ctrl+D на пустой строке: ввод закончен
    Eof,
    /// Ctrl+L: очистить экран и перерисовать строку
    ClearScreen,
}

/// Редактор строки ввода: буфер, курсор и история.
///
/// Сам ничего не печатает: `handle` меняет состояние, а `render` описывает,
/// как вывести строку. Так его можно проверить без экрана и клавиатуры
pub struct LineEditor {
    // char, а не байты: курсор двигается по символам, а 'ж' в UTF-8 занимает 2 байта
    buffer: Vec<char>,
    cursor: usize,
    max_len: usize,
    history: Vec<String>,
    // какая строка истории сейчас в буфере: None - новая строка
    history_pos: Option<usize>,
    // новая строка, которую набирали до того, как начали листать историю
    draft: Vec<char>,
}

impl LineEditor {
    /// Редактор строк не длиннее `max_len` символов
    pub fn new(max_len: usize) -> Self {
        LineEditor {
            buffer: Vec::new(),
            cursor: 0,
            max_len,
            history: Vec::new(),
            history_pos: None,
            draft: Vec::new(),
        }
    }

    pub fn set_max_len(&mut self, max_len: usize) {
        self.max_len = max_len;
    }

    /// Текущее содержимое строки
    pub fn line(&self) -> String {
        self.buffer.iter().collect()
    }

    /// Позиция курсора в символах от начала строки
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Введённые строки, от старых к новым
    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn handle(&mut self, key: Key) -> Event {
        match key {
            Key::Char(c) => self.insert(c),
            Key::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.buffer.remove(self.cursor);
                }
            }
            Key::Delete => self.delete(),
            Key::Left => self.cursor = self.cursor.saturating_sub(1),
            Key::Right => self.cursor = (self.cursor + 1).min(self.buffer.len()),
            Key::Home | Key::Ctrl('a') => self.cursor = 0,
            Key::End | Key::Ctrl('e') => self.cursor = self.buffer.len(),
            Key::Up => self.history_prev(),
            Key::Down => self.history_next(),
            // стереть всё до курсора, как в bash
            Key::Ctrl('u') => {
                self.buffer.drain(..self.cursor);
                self.cursor = 0;
            }
            // TODO: Ctrl+W - стереть слово перед курсором
            Key::Enter => return self.submit(),
            Key::Ctrl('c') => {
                self.cursor = self.buffer.len();
                return Event::Interrupt;
            }
            Key::Ctrl('d') => {
                if self.buffer.is_empty() {
                    return Event::Eof;
                }
                self.delete();
            }
            Key::Ctrl('l') => return Event::ClearScreen,
            // TODO: автодополнение имён команд по Tab
            Key::Tab | Key::Ctrl(_) => {}
        }
        Event::Continue
    }

    fn insert(&mut self, c: char) {
        if self.buffer.len() < self.max_len {
            self.buffer.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    fn submit(&mut self) -> Event {
        let line = self.line();
        // пустые строки и повтор предыдущей в историю не попадают
        if !line.trim().is_empty() && self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        self.clear();
        Event::Submit(line)
    }

    /// Очищает строку. История остаётся
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history_pos = None;
        self.draft.clear();
    }

    fn history_prev(&mut self) {
        let pos = match self.history_pos {
            None if self.history.is_empty() => return,
            None => {
                self.draft = core::mem::take(&mut self.buffer);
                self.history.len() - 1
            }
            Some(0) => return,
            Some(pos) => pos - 1,
        };
        self.history_pos = Some(pos);
        self.load(self.history[pos].chars().collect());
    }

    fn history_next(&mut self) {
        let Some(pos) = self.history_pos else {
            return;
        };
        if pos + 1 < self.history.len() {
            self.history_pos = Some(pos + 1);
            self.load(self.history[pos + 1].chars().collect());
        } else {
            // после самой новой строки истории - то, что набирали до неё
            self.history_pos = None;
            let draft = core::mem::take(&mut self.draft);
            self.load(draft);
        }
    }

    fn load(&mut self, mut line: Vec<char>) {
        line.truncate(self.max_len);
        self.buffer = line;
        self.cursor = self.buffer.len();
    }

    /// Выводит строку поверх текущей: возврат в начало, приглашение, текст,
    /// стирание остатка старой строки и возврат курсора на место
    pub fn render(&self, prompt: &str, out: &mut impl fmt::Write) -> fmt::Result {
        write!(out, "\r{}", prompt)?;
        for c in &self.buffer {
            out.write_char(*c)?;
        }
        out.write_str("\x1b[K")?;
        let back = self.buffer.len() - self.cursor;
        if back > 0 {
            write!(out, "\x1b[{}D", back)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
