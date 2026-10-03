use super::*;
use x86_64::instructions::interrupts;

// Тесты пишут в общий глобальный WRITER, поэтому каждый тест держит
// блокировку на всё время записи и проверки и начинает с '\n',
// чтобы не зависеть от того, что осталось на экране от предыдущих.
// Блокировка берётся с выключенными прерываниями: иначе обработчик
// таймера попытается напечатать '.' и зависнет на том же WRITER.

fn assert_row_eq(writer: &Writer, row: usize, expected: &[u8]) {
    for col in 0..BUFFER_WIDTH {
        let actual = writer.buffer.chars[row][col].read().ascii_character;
        let expected = expected.get(col).copied().unwrap_or(b' ');
        assert_eq!(actual, expected, "row {}, col {}", row, col);
    }
}

#[test_case]
fn test_color_code_encoding() {
    // старшие 4 бита - фон, младшие - цвет символа
    assert_eq!(ColorCode::new(Color::Yellow, Color::Blue), ColorCode(0x1e));
    assert_eq!(ColorCode::new(Color::LightGreen, Color::Black), ColorCode(0x0a));
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
        // 'ё' в UTF-8 занимает 2 байта, '\t' - непечатаемый символ
        writer.write_string("\nё\tz\n");
        assert_row_eq(&writer, BUFFER_HEIGHT - 2, &[0xfe, 0xfe, 0xfe, b'z']);
    });
}

#[test_case]
fn test_written_char_uses_writer_color() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.write_string("\nx");
        let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 1][0].read();
        assert_eq!(screen_char.color_code, writer.color_code);
        writer.write_byte(b'\n');
    });
}
