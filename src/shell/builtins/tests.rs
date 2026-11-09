use super::*;
use crate::shell::{STATUS_NOT_FOUND, STATUS_USAGE};
use crate::tty::Tty;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use futures_util::stream;

// reboot и poweroff не проверяются: они не возвращаются, а QEMU в тестах
// запущен с -no-reboot и при перезагрузке или выключении просто завершится

fn shell() -> Shell {
    Shell::new(Tty::with_keys(Box::new(stream::iter(Vec::new()))))
}

/// Выполняет строку и возвращает код возврата и вывод
fn run(shell: &mut Shell, line: &str) -> (i32, String) {
    let mut out = String::new();
    let status = {
        let mut future = pin!(shell.execute(line, &mut out));
        let mut context = Context::from_waker(Waker::noop());
        let Poll::Ready(status) = future.as_mut().poll(&mut context) else {
            panic!("command is waiting");
        };
        status
    };
    (status, out)
}

#[test_case]
fn test_command_names_unique() {
    for (i, a) in COMMANDS.iter().enumerate() {
        for b in &COMMANDS[i + 1..] {
            assert_ne!(a.name, b.name);
        }
    }
}

#[test_case]
fn test_every_command_has_help() {
    for command in COMMANDS {
        assert!(!command.help.is_empty(), "{} has no help", command.name);
    }
}

#[test_case]
fn test_find() {
    assert_eq!(find("echo").map(|c| c.name), Some("echo"));
    assert!(find("no-such-command").is_none());
    // имена чувствительны к регистру
    assert!(find("ECHO").is_none());
}

#[test_case]
fn test_expected_commands_exist() {
    for name in [
        "help", "echo", "clear", "history", "uptime", "sleep", "exit", "reboot", "poweroff",
    ] {
        assert!(find(name).is_some(), "{} missing", name);
    }
}

#[test_case]
fn test_help_lists_every_command() {
    let (status, out) = run(&mut shell(), "help");
    assert_eq!(status, 0);
    assert_eq!(out.lines().count(), COMMANDS.len());
    for (line, command) in out.lines().zip(COMMANDS) {
        assert!(line.starts_with(command.name));
        assert!(line.ends_with(command.help));
    }
}

#[test_case]
fn test_echo() {
    assert_eq!(
        run(&mut shell(), "echo hello world"),
        (0, "hello world\n".into())
    );
}

#[test_case]
fn test_echo_without_args_prints_newline() {
    assert_eq!(run(&mut shell(), "echo"), (0, "\n".into()));
}

#[test_case]
fn test_echo_joins_args_with_single_space() {
    // пробелы между словами схлопываются, внутри кавычек - нет
    assert_eq!(
        run(&mut shell(), "echo a    b 'c  d'"),
        (0, "a b c  d\n".into())
    );
}

#[test_case]
fn test_echo_n_skips_newline() {
    assert_eq!(run(&mut shell(), "echo -n hi"), (0, "hi".into()));
}

#[test_case]
fn test_echo_n_only_as_first_arg() {
    assert_eq!(run(&mut shell(), "echo hi -n"), (0, "hi -n\n".into()));
}

#[test_case]
fn test_clear_outputs_escape_sequence() {
    assert_eq!(run(&mut shell(), "clear"), (0, "\x1b[2J\x1b[H".into()));
}

#[test_case]
fn test_history_empty() {
    // execute не читает строк из терминала, поэтому история пуста
    assert_eq!(run(&mut shell(), "history"), (0, String::new()));
}

/// Докручивает тики так, чтобы uptime был ровно `seconds` секунд или больше,
/// и возвращает итоговое число секунд
fn advance_to_whole_second(seconds: u64) -> u64 {
    // в unit-тестах прерывания выключены, тики меняет только тест
    let target = seconds * time::TICKS_PER_SECOND;
    while time::ticks() < target || !time::ticks().is_multiple_of(time::TICKS_PER_SECOND) {
        time::tick();
    }
    time::ticks() / time::TICKS_PER_SECOND
}

#[test_case]
fn test_uptime_format() {
    let (status, out) = run(&mut shell(), "uptime");
    assert_eq!(status, 0);
    // up ЧЧ:ММ:СС
    assert_eq!(out.len(), "up 00:00:00\n".len(), "{:?}", out);
    assert!(out.starts_with("up "), "{:?}", out);
}

#[test_case]
fn test_uptime_shows_hours_minutes_seconds() {
    let seconds = advance_to_whole_second(62);
    let (_, out) = run(&mut shell(), "uptime");
    let expected = alloc::format!(
        "up {:02}:{:02}:{:02}\n",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    );
    assert_eq!(out, expected);
}

#[test_case]
fn test_sleep_zero_completes_at_once() {
    assert_eq!(run(&mut shell(), "sleep 0"), (0, String::new()));
}

#[test_case]
fn test_sleep_waits_for_ticks() {
    let mut shell = shell();
    let mut out = String::new();
    let mut future = pin!(shell.execute("sleep 0.05", &mut out));
    let mut context = Context::from_waker(Waker::noop());
    // 50 мс - это 5 тиков по 10 мс
    for _ in 0..5 {
        assert!(future.as_mut().poll(&mut context).is_pending());
        time::tick();
    }
    assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(0));
}

#[test_case]
fn test_sleep_without_argument() {
    let (status, out) = run(&mut shell(), "sleep");
    assert_eq!(status, STATUS_USAGE);
    assert_eq!(out, "usage: sleep SECONDS\n");
}

#[test_case]
fn test_sleep_with_too_many_arguments() {
    assert_eq!(run(&mut shell(), "sleep 1 2").0, STATUS_USAGE);
}

#[test_case]
fn test_sleep_with_bad_argument() {
    let (status, out) = run(&mut shell(), "sleep soon");
    assert_eq!(status, STATUS_USAGE);
    assert_eq!(out, "sleep: invalid time interval 'soon'\n");
}

#[test_case]
fn test_sleep_releases_slot_after_completion() {
    let before = time::sleeping();
    run(&mut shell(), "sleep 0");
    assert_eq!(time::sleeping(), before);
}

#[test_case]
fn test_parse_whole_seconds() {
    assert_eq!(parse_seconds("0"), Some(Duration::ZERO));
    assert_eq!(parse_seconds("2"), Some(Duration::from_secs(2)));
    assert_eq!(parse_seconds("120"), Some(Duration::from_secs(120)));
}

#[test_case]
fn test_parse_fractional_seconds() {
    assert_eq!(parse_seconds("0.5"), Some(Duration::from_millis(500)));
    assert_eq!(parse_seconds("1.25"), Some(Duration::from_millis(1250)));
    assert_eq!(parse_seconds("0.125"), Some(Duration::from_millis(125)));
    assert_eq!(parse_seconds(".5"), Some(Duration::from_millis(500)));
    assert_eq!(parse_seconds("3."), Some(Duration::from_secs(3)));
}

#[test_case]
fn test_parse_drops_below_milliseconds() {
    assert_eq!(parse_seconds("0.0019"), Some(Duration::from_millis(1)));
}

#[test_case]
fn test_parse_rejects_bad_input() {
    for bad in [
        "",
        ".",
        "-1",
        "1.2.3",
        "1e3",
        "abc",
        "1s",
        " 1",
        "99999999999999999999",
    ] {
        assert_eq!(parse_seconds(bad), None, "{:?}", bad);
    }
}

#[test_case]
fn test_exit_without_args_uses_last_status() {
    let mut shell = shell();
    run(&mut shell, "no-such-command");
    let (status, out) = run(&mut shell, "exit");
    // execute сам не меняет status: его обновляет run, поэтому здесь 0
    assert_eq!(status, 0);
    assert_eq!(out, "");
    assert!(shell.exit_requested());
}

#[test_case]
fn test_exit_with_code() {
    let mut shell = shell();
    assert_eq!(run(&mut shell, "exit 3"), (3, String::new()));
    assert!(shell.exit_requested());
}

#[test_case]
fn test_exit_with_negative_code() {
    let mut shell = shell();
    assert_eq!(run(&mut shell, "exit -1").0, -1);
}

#[test_case]
fn test_exit_with_bad_code() {
    let mut shell = shell();
    let (status, out) = run(&mut shell, "exit abc");
    assert_eq!(status, STATUS_USAGE);
    assert_eq!(out, "exit: abc: numeric argument required\n");
    assert!(!shell.exit_requested());
}

#[test_case]
fn test_exit_with_too_many_args() {
    let mut shell = shell();
    let (status, out) = run(&mut shell, "exit 1 2");
    assert_eq!(status, STATUS_USAGE);
    assert_eq!(out, "exit: too many arguments\n");
    assert!(!shell.exit_requested());
}

#[test_case]
fn test_unknown_command_status() {
    assert_eq!(run(&mut shell(), "frobnicate").0, STATUS_NOT_FOUND);
}
