use super::*;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

// Клавиши берутся из заранее заданного списка (stream::iter), поэтому
// read_line выполняется за один опрос, без клавиатуры и прерываний.
// Вывод проверяется по экрану

const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;

/// Строка экрана без пробелов в конце
fn screen_row(row: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for col in 0..vga::BUFFER_WIDTH {
        let offset = row * vga::BUFFER_WIDTH + col;
        out.push(unsafe { VGA_BUFFER.add(offset).read_volatile() } as u8);
    }
    while out.last() == Some(&b' ') {
        out.pop();
    }
    out
}

/// Строка экрана над курсором: туда попадает введённая строка после Enter
fn row_above_cursor() -> Vec<u8> {
    screen_row(vga::BUFFER_HEIGHT - 2)
}

/// Выполняет future, который не ждёт ничего внешнего
fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("future is waiting for input"),
    }
}

fn tty(keys: Vec<Key>) -> Tty {
    Tty::with_keys(Box::new(stream::iter(keys)))
}

fn chars(s: &str) -> Vec<Key> {
    s.chars().map(Key::Char).collect()
}

fn keys(parts: &[&[Key]]) -> Vec<Key> {
    parts.concat()
}

#[test_case]
fn test_read_line_returns_typed_line() {
    print!("\n");
    let mut tty = tty(keys(&[&chars("ls -l"), &[Key::Enter]]));
    assert_eq!(
        run(tty.read_line("> ")),
        ReadLine::Line("ls -l".to_string())
    );
}

#[test_case]
fn test_read_line_shows_prompt_and_line() {
    print!("\n");
    let mut tty = tty(keys(&[&chars("echo hi"), &[Key::Enter]]));
    run(tty.read_line("rustos$ "));
    assert_eq!(row_above_cursor(), b"rustos$ echo hi");
    // после Enter курсор на новой пустой строке
    assert_eq!(screen_row(vga::BUFFER_HEIGHT - 1), b"");
}

#[test_case]
fn test_read_line_shows_edited_line() {
    print!("\n");
    let mut tty = tty(keys(&[
        &chars("helo"),
        &[Key::Left],
        &chars("l"),
        &[Key::End, Key::Backspace, Key::Enter],
    ]));
    assert_eq!(run(tty.read_line("> ")), ReadLine::Line("hell".to_string()));
    assert_eq!(row_above_cursor(), b"> hell");
}

#[test_case]
fn test_read_line_interrupted() {
    print!("\n");
    let mut tty = tty(keys(&[&chars("abc"), &[Key::Home, Key::Ctrl('c')]]));
    assert_eq!(run(tty.read_line("> ")), ReadLine::Interrupted);
    // ^C печатается после всей строки, даже если курсор был в начале
    assert_eq!(row_above_cursor(), b"> abc^C");
}

#[test_case]
fn test_line_after_interrupt_starts_empty() {
    print!("\n");
    let mut tty = tty(keys(&[
        &chars("abc"),
        &[Key::Ctrl('c')],
        &chars("x"),
        &[Key::Enter],
    ]));
    assert_eq!(run(tty.read_line("> ")), ReadLine::Interrupted);
    assert_eq!(run(tty.read_line("> ")), ReadLine::Line("x".to_string()));
}

#[test_case]
fn test_read_line_eof_on_ctrl_d() {
    print!("\n");
    let mut tty = tty(alloc::vec![Key::Ctrl('d')]);
    assert_eq!(run(tty.read_line("> ")), ReadLine::Eof);
}

#[test_case]
fn test_read_line_eof_when_keys_end() {
    print!("\n");
    let mut tty = tty(chars("unfinished"));
    assert_eq!(run(tty.read_line("> ")), ReadLine::Eof);
}

#[test_case]
fn test_history_kept_between_lines() {
    print!("\n");
    let mut tty = tty(keys(&[
        &chars("first"),
        &[Key::Enter],
        &[Key::Up, Key::Enter],
    ]));
    run(tty.read_line("> "));
    assert_eq!(
        run(tty.read_line("> ")),
        ReadLine::Line("first".to_string())
    );
    assert_eq!(tty.history(), &["first".to_string()]);
}

#[test_case]
fn test_ctrl_l_clears_screen_and_redraws_line() {
    print!("\nsomething above");
    print!("\n");
    let mut tty = tty(keys(&[&chars("ab"), &[Key::Ctrl('l'), Key::Enter]]));
    run(tty.read_line("> "));
    assert_eq!(row_above_cursor(), b"> ab");
    for row in 0..vga::BUFFER_HEIGHT - 2 {
        assert_eq!(screen_row(row), b"", "row {}", row);
    }
}

#[test_case]
fn test_line_fits_screen_width() {
    print!("\n");
    let prompt = "p".repeat(70);
    let mut tty = tty(keys(&[&chars("0123456789abcdef"), &[Key::Enter]]));
    // 80 колонок - 70 на приглашение - 1 под курсор
    assert_eq!(
        run(tty.read_line(&prompt)),
        ReadLine::Line("012345678".to_string())
    );
}
