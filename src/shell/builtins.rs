use super::{CommandResult, STATUS_USAGE, Shell};
use crate::arch::x86_64::power;
use crate::time;
use alloc::boxed::Box;
use core::fmt::Write;
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;

pub type CommandFuture<'a> = Pin<Box<dyn Future<Output = CommandResult> + 'a>>;

#[derive(Clone, Copy)]
pub enum Run {
    Sync(fn(&mut Shell, &[&str], &mut dyn Write) -> CommandResult),
    Async(for<'a> fn(&'a mut Shell, &'a [&'a str], &'a mut dyn Write) -> CommandFuture<'a>),
}

pub struct Command {
    pub name: &'static str,
    pub help: &'static str,
    pub run: Run,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "help",
        help: "list commands",
        run: Run::Sync(help),
    },
    Command {
        name: "echo",
        help: "print arguments (-n: no newline)",
        run: Run::Sync(echo),
    },
    Command {
        name: "clear",
        help: "clear the screen",
        run: Run::Sync(clear),
    },
    Command {
        name: "history",
        help: "show entered lines",
        run: Run::Sync(history),
    },
    Command {
        name: "uptime",
        help: "time since boot",
        run: Run::Sync(uptime),
    },
    Command {
        name: "sleep",
        help: "wait for N seconds (N may be fractional)",
        run: Run::Async(sleep),
    },
    Command {
        name: "exit",
        help: "exit the shell [code]",
        run: Run::Sync(exit),
    },
    Command {
        name: "reboot",
        help: "restart the computer",
        run: Run::Sync(reboot),
    },
    Command {
        name: "poweroff",
        help: "turn the computer off",
        run: Run::Sync(poweroff),
    },
    Command {
        name: "date",
        help: "print date and time (-u: UTC, +%s: Unix time)",
        run: Run::Sync(date),
    },
    // TODO (статья 13): uname - имя ОС, с -a ещё версия и архитектура
    // TODO: true и false - коды 0 и 1, пригодятся для && и ||
    // TODO: cal - календарь текущего месяца (days_in_month и weekday первого числа)
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

// TODO: uptime --raw - секунды с миллисекундами, `up 12.340 s` (Duration::subsec_millis)
fn uptime(_shell: &mut Shell, _args: &[&str], out: &mut dyn Write) -> CommandResult {
    let seconds = time::uptime().as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    writeln!(out, "up {:02}:{:02}:{:02}", hours, minutes, seconds)?;
    Ok(0)
}

fn sleep<'a>(
    _shell: &'a mut Shell,
    args: &'a [&'a str],
    out: &'a mut dyn Write,
) -> CommandFuture<'a> {
    Box::pin(async move {
        let [arg] = args else {
            writeln!(out, "usage: sleep SECONDS")?;
            return Ok(STATUS_USAGE);
        };
        let Some(duration) = parse_seconds(arg) else {
            writeln!(out, "sleep: invalid time interval '{}'", arg)?;
            return Ok(STATUS_USAGE);
        };
        time::sleep(duration).await;
        Ok(0)
    })
}

pub fn parse_seconds(s: &str) -> Option<Duration> {
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let seconds: u64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };

    let mut millis = 0;
    for (i, digit) in fraction.bytes().take(3).enumerate() {
        millis += (digit - b'0') as u64 * 10u64.pow(2 - i as u32);
    }
    Some(Duration::from_secs(seconds) + Duration::from_millis(millis))
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

// TODO: date -d @1793095200 - дата для заданного Unix-времени
// TODO: date -s 2026-12-31T23:59:50 - установить время через set_time. Что делать с RTC?
fn date(_shell: &mut Shell, args: &[&str], out: &mut dyn Write) -> CommandResult {
    let tz = match args {
        [] => time::LOCAL,
        ["-u"] => time::UTC,
        ["+%s"] => {
            writeln!(out, "{}", time::now())?;
            return Ok(0);
        }
        _ => {
            writeln!(out, "usage: date [-u | +%s]")?;
            return Ok(STATUS_USAGE);
        }
    };
    writeln!(out, "{}", format_date(&time::now_in(tz), tz))?;
    Ok(0)
}

pub fn format_date(date: &time::DateTime, tz: time::TimeZone) -> alloc::string::String {
    alloc::format!(
        "{} {} {:2} {:02}:{:02}:{:02} {} {}",
        date.weekday_name(),
        date.month_name(),
        date.day,
        date.hour,
        date.minute,
        date.second,
        tz.name,
        date.year
    )
}

#[cfg(test)]
mod tests;
