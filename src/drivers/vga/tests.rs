use super::*;
use crate::println;
use x86_64::instructions::interrupts;

// Тесты пишут в общий глобальный WRITER, поэтому каждый тест держит
// блокировку на всё время записи и проверки и начинает с '\n',
// чтобы не зависеть от того, что осталось на экране от предыдущих.
// Блокировка берётся с выключенными прерываниями: иначе обработчик
// прерывания, который печатает через println!, зависнет на том же WRITER.

fn assert_row_eq(writer: &Writer, row: usize, expected: &[u8]) {
    for col in 0..BUFFER_WIDTH {
        let actual = writer.buffer.read(row, col).ascii_character;
        let expected = expected.get(col).copied().unwrap_or(b' ');
        assert_eq!(actual, expected, "row {}, col {}", row, col);
    }
}

#[test_case]
fn test_color_code_encoding() {
    // старшие 4 бита - фон, младшие - цвет символа
    assert_eq!(ColorCode::new(Color::Yellow, Color::Blue), ColorCode(0x1e));
    assert_eq!(
        ColorCode::new(Color::LightGreen, Color::Black),
        ColorCode(0x0a)
    );
    assert_eq!(ColorCode::new(Color::White, Color::White), ColorCode(0xff));
}

#[test_case]
fn test_memory_layout() {
    // раскладка должна совпадать с текстовым VGA-буфером: 2 байта на символ
    use core::mem::size_of;
    assert_eq!(size_of::<ColorCode>(), 1);
    assert_eq!(size_of::<ScreenChar>(), 2);
    assert_eq!(size_of::<Buffer>(), BUFFER_WIDTH * BUFFER_HEIGHT * 2);
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    interrupts::without_interrupts(|| {
        use core::fmt::Write;

        let s = "Some test string that fits on a single line";
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).unwrap();
        assert_row_eq(&writer, BUFFER_HEIGHT - 2, s.as_bytes());
    });
}

#[test_case]
fn test_newline_resets_column() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabc\n");
        assert_eq!(writer.column_position, 0);
        assert_row_eq(&writer, BUFFER_HEIGHT - 2, b"abc");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"");
    });
}

#[test_case]
fn test_long_line_wraps() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_byte(b'\n');
        for _ in 0..BUFFER_WIDTH {
            writer.write_byte(b'a');
        }
        writer.write_byte(b'b');

        assert_row_eq(&writer, BUFFER_HEIGHT - 2, &[b'a'; BUFFER_WIDTH]);
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"b");
        assert_eq!(writer.column_position, 1);
        writer.write_byte(b'\n');
    });
}

#[test_case]
fn test_screen_scrolls() {
    interrupts::without_interrupts(|| {
        use core::fmt::Write;

        let mut writer = WRITER.lock();
        for i in 0..BUFFER_HEIGHT {
            writeln!(writer, "line {}", i).unwrap();
        }
        // последняя строка пустая, строка 0 уехала за верх экрана
        assert_row_eq(&writer, 0, b"line 1");
        assert_row_eq(&writer, BUFFER_HEIGHT - 2, b"line 24");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"");
    });
}

#[test_case]
fn test_non_ascii_replaced() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        // 'ё' в UTF-8 занимает 2 байта, но это один символ и один ■.
        // '\t' - непечатаемый символ
        writer.write_string("\nё\tz🦀\n");
        assert_row_eq(&writer, BUFFER_HEIGHT - 2, &[0xfe, 0xfe, b'z', 0xfe]);
    });
}

#[test_case]
fn test_written_char_uses_writer_color() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nx");
        let screen_char = writer.buffer.read(BUFFER_HEIGHT - 1, 0);
        assert_eq!(screen_char.color_code, writer.color_code);
        writer.write_byte(b'\n');
    });
}

// escape-последовательности и курсор

/// Позиция аппаратного курсора из регистров контроллера CRT
fn hardware_cursor() -> u16 {
    use x86_64::instructions::port::Port;

    let mut index = Port::<u8>::new(0x3d4);
    let mut data = Port::<u8>::new(0x3d5);
    unsafe {
        index.write(0x0f);
        let low = data.read() as u16;
        index.write(0x0e);
        let high = data.read() as u16;
        high << 8 | low
    }
}

#[test_case]
fn test_carriage_return_moves_to_line_start() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nhello\rj");
        assert_eq!(writer.column_position, 1);
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"jello");
    });
}

#[test_case]
fn test_cursor_left() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabcd\x1b[2DX");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"abXd");
    });
}

#[test_case]
fn test_cursor_left_without_param_moves_one() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabc\x1b[DX");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"abX");
    });
}

#[test_case]
fn test_cursor_left_stops_at_start() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nab\x1b[10DX");
        assert_eq!(writer.column_position, 1);
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"Xb");
    });
}

#[test_case]
fn test_cursor_right() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabcd\r\x1b[2CX");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"abXd");
    });
}

#[test_case]
fn test_cursor_right_stops_at_last_column() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\n\x1b[999C");
        assert_eq!(writer.column_position, BUFFER_WIDTH - 1);
        writer.write_byte(b'\n');
    });
}

#[test_case]
fn test_clear_to_end_of_line() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nhello world\x1b[6D\x1b[K");
        assert_eq!(writer.column_position, 5);
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"hello");
    });
}

#[test_case]
fn test_redraw_line_like_line_editor() {
    // так LineEditor::render перерисовывает строку после Backspace
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\n> hello");
        writer.write_string("\r> hell\x1b[K");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"> hell");
        assert_eq!(writer.column_position, 6);
    });
}

#[test_case]
fn test_clear_screen() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabove\nbelow");
        writer.write_string("\x1b[2J\x1b[H");
        for row in 0..BUFFER_HEIGHT {
            assert_row_eq(&writer, row, b"");
        }
        assert_eq!(writer.column_position, 0);
    });
}

#[test_case]
fn test_unsupported_sequences_swallowed() {
    // цвета (m), в том числе с несколькими параметрами, очистка экрана
    // с другим параметром; у последовательности не CSI (ESC X) пропадает X
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\na\x1b[31mb\x1b[1Jc\x1bXd\x1b[1;31me\x1b[0m");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"abcde");
    });
}

#[test_case]
fn test_sequence_split_between_writes() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabc\x1b");
        writer.write_string("[");
        writer.write_string("2");
        writer.write_string("DX");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"aXc");
    });
}

#[test_case]
fn test_broken_sequence_recovers() {
    // управляющий символ посреди последовательности обрывает её
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\na\x1b[\x01b");
        assert_eq!(writer.escape, Escape::Ground);
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"ab");
    });
}

#[test_case]
fn test_writer_returns_to_text_after_sequence() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\n\x1b[K");
        assert_eq!(writer.escape, Escape::Ground);
        writer.write_string("ok");
        assert_row_eq(&writer, BUFFER_HEIGHT - 1, b"ok");
    });
}

#[test_case]
fn test_hardware_cursor_follows_column() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nabc");
        let last_row = ((BUFFER_HEIGHT - 1) * BUFFER_WIDTH) as u16;
        assert_eq!(hardware_cursor(), last_row + 3);
        writer.write_string("\x1b[2D");
        assert_eq!(hardware_cursor(), last_row + 1);
        writer.write_string("\n");
        assert_eq!(hardware_cursor(), last_row);
    });
}

#[test_case]
fn test_hardware_cursor_stays_on_screen_at_line_end() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_byte(b'\n');
        for _ in 0..BUFFER_WIDTH {
            writer.write_byte(b'a');
        }
        writer.write_string("");
        // колонка 80 - за краем экрана, курсор остаётся в последней колонке
        let last_cell = (BUFFER_HEIGHT * BUFFER_WIDTH - 1) as u16;
        assert_eq!(hardware_cursor(), last_cell);
        writer.write_byte(b'\n');
    });
}
