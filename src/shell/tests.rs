use super::*;
use crate::drivers::vga;
use crate::print;
use crate::tty::Key;
use alloc::boxed::Box;
use alloc::string::ToString;
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use futures_util::stream;

fn shell_with_keys(keys: Vec<Key>) -> Shell {
    Shell::new(Tty::with_keys(Box::new(stream::iter(keys))))
}

fn shell() -> Shell {
    shell_with_keys(Vec::new())
}

/// Клавиши для набора строки и Enter
fn line(s: &str) -> Vec<Key> {
    let mut keys: Vec<Key> = s.chars().map(Key::Char).collect();
    keys.push(Key::Enter);
    keys
}

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("future is waiting for input"),
    }
}

/// Строка экрана без пробелов в конце
fn screen_row(row: usize) -> String {
    const VGA_BUFFER: *const u16 = 0xb8000 as *const u16;
    let mut out = String::new();
    for col in 0..vga::BUFFER_WIDTH {
        let offset = row * vga::BUFFER_WIDTH + col;
        out.push(unsafe { VGA_BUFFER.add(offset).read_volatile() } as u8 as char);
    }
    out.trim_end().to_string()
}

fn execute(shell: &mut Shell, line: &str) -> (i32, String) {
    let mut out = String::new();
    let status = run(shell.execute(line, &mut out));
    (status, out)
}

// execute

#[test_case]
fn test_new_shell_status_is_zero() {
    let shell = shell();
    assert_eq!(shell.status(), 0);
    assert!(!shell.exit_requested());
}

#[test_case]
fn test_empty_line_does_nothing() {
    assert_eq!(execute(&mut shell(), ""), (0, String::new()));
    assert_eq!(execute(&mut shell(), "   "), (0, String::new()));
}

#[test_case]
fn test_unknown_command() {
    let (status, out) = execute(&mut shell(), "frobnicate --now");
    assert_eq!(status, STATUS_NOT_FOUND);
    assert_eq!(out, "rustos: frobnicate: command not found\n");
}

#[test_case]
fn test_parse_error() {
    let (status, out) = execute(&mut shell(), "echo 'oops");
    assert_eq!(status, STATUS_USAGE);
    assert_eq!(out, "rustos: unclosed quote '\n");
}

#[test_case]
fn test_command_name_can_be_quoted() {
    assert_eq!(execute(&mut shell(), "'echo' hi"), (0, "hi\n".to_string()));
}

#[test_case]
fn test_empty_command_name() {
    // "" - пустое слово, а не пустая строка
    let (status, out) = execute(&mut shell(), "''");
    assert_eq!(status, STATUS_NOT_FOUND);
    assert_eq!(out, "rustos: : command not found\n");
}

#[test_case]
fn test_failed_output_is_command_error() {
    struct Broken;
    impl fmt::Write for Broken {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }
    assert_eq!(run(shell().execute("echo hi", &mut Broken)), 1);
}

// run

#[test_case]
fn test_run_ends_on_eof() {
    let mut shell = shell_with_keys(Vec::new());
    run(shell.run());
    assert!(!shell.exit_requested());
}

#[test_case]
fn test_run_ends_on_exit() {
    let mut keys = line("exit 5");
    // после exit оболочка больше не читает строки
    keys.extend(line("echo unreachable"));
    let mut shell = shell_with_keys(keys);
    print!("\n");
    run(shell.run());
    assert!(shell.exit_requested());
    assert_eq!(shell.status(), 5);
    assert_eq!(shell.history().len(), 1);
}

#[test_case]
fn test_run_prints_command_output() {
    let mut shell = shell_with_keys(line("echo from shell"));
    print!("\n");
    run(shell.run());
    let last = vga::BUFFER_HEIGHT - 1;
    // приглашение с командой, вывод команды, новое приглашение и \n после Ctrl+D
    assert_eq!(screen_row(last - 3), "rustos$ echo from shell");
    assert_eq!(screen_row(last - 2), "from shell");
    assert_eq!(screen_row(last - 1), "rustos$");
}

#[test_case]
fn test_run_updates_status() {
    let mut shell = shell_with_keys(line("frobnicate"));
    print!("\n");
    run(shell.run());
    assert_eq!(shell.status(), STATUS_NOT_FOUND);
}

#[test_case]
fn test_empty_line_keeps_status() {
    let mut keys = line("frobnicate");
    keys.extend(line(""));
    let mut shell = shell_with_keys(keys);
    print!("\n");
    run(shell.run());
    assert_eq!(shell.status(), STATUS_NOT_FOUND);
}

#[test_case]
fn test_ctrl_c_sets_interrupted_status() {
    let mut keys: Vec<Key> = "abc".chars().map(Key::Char).collect();
    keys.push(Key::Ctrl('c'));
    let mut shell = shell_with_keys(keys);
    print!("\n");
    run(shell.run());
    assert_eq!(shell.status(), STATUS_INTERRUPTED);
}

#[test_case]
fn test_exit_without_code_uses_last_status() {
    let mut keys = line("frobnicate");
    keys.extend(line("exit"));
    let mut shell = shell_with_keys(keys);
    print!("\n");
    run(shell.run());
    assert_eq!(shell.status(), STATUS_NOT_FOUND);
}

#[test_case]
fn test_history_command_shows_entered_lines() {
    let mut keys = line("echo one");
    keys.extend(line("history"));
    let mut shell = shell_with_keys(keys);
    print!("\n");
    run(shell.run());
    let last = vga::BUFFER_HEIGHT - 1;
    assert_eq!(screen_row(last - 3), "    1  echo one");
    assert_eq!(screen_row(last - 2), "    2  history");
}
