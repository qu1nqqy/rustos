use super::{CommandResult, STATUS_USAGE, Shell};
use crate::arch::x86_64::power;
use crate::time;
use core::fmt::Write;
pub struct Command {
    pub name: &'static str,
    pub help: &'static str,
    pub run: fn(&mut Shell, &[&str], &mut dyn Write) -> CommandResult,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "help",
        help: "list commands",
        run: help,
    },
    Command {
        name: "echo",
        help: "print arguments (-n: no newline)",
        run: echo,
    },
    Command {
        name: "clear",
        help: "clear the screen",
        run: clear,
    },
    Command {
        name: "history",
        help: "show entered lines",
        run: history,
    },
    Command {
        name: "uptime",
        help: "time since boot",
        run: uptime,
    },
    Command {
        name: "exit",
        help: "exit the shell [code]",
        run: exit,
    },
    Command {
        name: "reboot",
        help: "restart the computer",
        run: reboot,
    },
    Command {
        name: "poweroff",
        help: "turn the computer off",
        run: poweroff,
    },
    // TODO: uname - имя ОС, версия (env!("CARGO_PKG_VERSION")) и архитектура
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name == name)
}

fn help(_shell: &mut Shell, _args: &[&str], out: &mut dyn Write) -> CommandResult {
    for command in COMMANDS {
        writeln!(out, "{:<10} {}", command.name, command.help)?;
    }
    Ok(0)
}

fn echo(_shell: &mut Shell, args: &[&str], out: &mut dyn Write) -> CommandResult {
    let (newline, args) = match args.split_first() {
        Some((&"-n", rest)) => (false, rest),
        _ => (true, args),
    };
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            out.write_char(' ')?;
        }
        out.write_str(arg)?;
    }
    if newline {
        out.write_char('\n')?;
    }
    Ok(0)
}

fn clear(_shell: &mut Shell, _args: &[&str], out: &mut dyn Write) -> CommandResult {
    out.write_str("\x1b[2J\x1b[H")?;
    Ok(0)
}

fn history(shell: &mut Shell, _args: &[&str], out: &mut dyn Write) -> CommandResult {
    for (i, line) in shell.history().iter().enumerate() {
        writeln!(out, "{:>5}  {}", i + 1, line)?;
    }
    Ok(0)
}

fn uptime(_shell: &mut Shell, _args: &[&str], out: &mut dyn Write) -> CommandResult {
    let ticks = time::ticks();
    // таймер не настроен и тикает с частотой ≈ 18.2 Гц
    let seconds = ticks * 10 / 182;
    writeln!(out, "up {} s ({} ticks)", seconds, ticks)?;
    Ok(0)
}

fn exit(shell: &mut Shell, args: &[&str], out: &mut dyn Write) -> CommandResult {
    let status = match args {
        // без аргумента - код последней команды
        [] => shell.status(),
        [code] => match code.parse::<i32>() {
            Ok(code) => code,
            Err(_) => {
                writeln!(out, "exit: {}: numeric argument required", code)?;
                return Ok(STATUS_USAGE);
            }
        },
        _ => {
            writeln!(out, "exit: too many arguments")?;
            return Ok(STATUS_USAGE);
        }
    };
    shell.request_exit();
    Ok(status)
}

fn reboot(_shell: &mut Shell, _args: &[&str], _out: &mut dyn Write) -> CommandResult {
    power::reboot();
}

fn poweroff(_shell: &mut Shell, _args: &[&str], _out: &mut dyn Write) -> CommandResult {
    power::poweroff();
}

#[cfg(test)]
mod tests;