use super::*;
use crate::shell::{STATUS_NOT_FOUND, STATUS_USAGE};
use crate::tty::Tty;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use futures_util::stream;

// reboot и poweroff не проверяются: они не возвращаются, а QEMU в тестах
// запущен с -no-reboot и при перезагрузке или выключении просто завершится

fn shell() -> Shell {
    Shell::new(Tty::with_keys(Box::new(stream::iter(Vec::new()))))
}

/// Выполняет строку и возвращает код возврата и вывод
fn run(shell: &mut Shell, line: &str) -> (i32, String) {
    let mut out = String::new();
    let status = shell.execute(line, &mut out);
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
        "help", "echo", "clear", "history", "uptime", "exit", "reboot", "poweroff",
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

#[test_case]
fn test_uptime_format() {
    let (status, out) = run(&mut shell(), "uptime");
    assert_eq!(status, 0);
    assert!(out.starts_with("up "), "{:?}", out);
    assert!(out.ends_with(" ticks)\n"), "{:?}", out);
}

#[test_case]
fn test_uptime_converts_ticks_to_seconds() {
    // в unit-тестах прерывания выключены, тики меняет только тест
    let before = time::ticks();
    for _ in 0..(182 - before % 182) {
        time::tick();
    }
    let ticks = time::ticks();
    let (_, out) = run(&mut shell(), "uptime");
    assert_eq!(
        out,
        alloc::format!("up {} s ({} ticks)\n", ticks / 182 * 10, ticks)
    );
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
