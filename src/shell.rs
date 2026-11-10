mod builtins;
mod parse;

pub use builtins::{COMMANDS, Command, CommandFuture, Run};
pub use parse::{ParseError, split};

use crate::console::Console;
use crate::tty::{ReadLine, Tty};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write};

const PROMPT: &str = "rustos$ ";

pub const STATUS_NOT_FOUND: i32 = 127;
pub const STATUS_INTERRUPTED: i32 = 130;
pub const STATUS_USAGE: i32 = 2;

pub struct Shell {
    tty: Tty,
    status: i32,
    exit_requested: bool,
}

impl Shell {
    pub fn new(tty: Tty) -> Self {
        Shell {
            tty,
            status: 0,
            exit_requested: false,
        }
    }

    pub async fn run(&mut self) {
        while !self.exit_requested {
            match self.tty.read_line(PROMPT).await {
                ReadLine::Line(line) => {
                    self.status = self.execute(&line, &mut Console).await;
                }
                ReadLine::Interrupted => self.status = STATUS_INTERRUPTED,
                ReadLine::Eof => break,
            }
        }
    }

    pub async fn execute(&mut self, line: &str, out: &mut dyn Write) -> i32 {
        // TODO: заменять $? на код последней команды, чтобы работал `echo $?`
        let words = match split(line) {
            Ok(words) => words,
            Err(e) => {
                let _ = writeln!(out, "rustos: {}", e);
                return STATUS_USAGE;
            }
        };

        let Some((name, args)) = words.split_first() else {
            return self.status;
        };
        let args: Vec<&str> = args.iter().map(String::as_str).collect();

        let Some(command) = builtins::find(name) else {
            let _ = writeln!(out, "rustos: {}: command not found", name);
            return STATUS_NOT_FOUND;
        };
        let result = match command.run {
            Run::Sync(run) => run(self, &args, out),
            Run::Async(run) => run(self, &args, out).await,
        };

        result.unwrap_or(1)
    }

    pub fn status(&self) -> i32 {
        self.status
    }

    pub fn history(&self) -> &[String] {
        self.tty.history()
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    pub fn into_tty(self) -> Tty {
        self.tty
    }

    fn request_exit(&mut self) {
        self.exit_requested = true
    }
}

pub type CommandResult = Result<i32, fmt::Error>;

#[cfg(test)]
mod tests;
