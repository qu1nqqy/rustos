use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    UnclosedQuote(char),
    TrailingBackslash,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnclosedQuote(c) => write!(f, "unclosed quote {}", c),
            ParseError::TrailingBackslash => write!(f, "trailing backslash"),
        }
    }
}

pub fn split(line : &str) -> Result<Vec<String>, ParseError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = line.chars();

    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => {
                if in_word {
                    words.push(core::mem::take(&mut word));
                    in_word = false;
                }
            },
            '\'' => {
                in_word = true;
                loop  {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => word.push(c),
                        None => return Err(ParseError::UnclosedQuote('\'')),
                    }
                }
            },
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\')) => word.push(c),
                            Some(c) => {
                                word.push('\\');
                                word.push(c);

                            },
                            None => return Err(ParseError::UnclosedQuote('"')),
                        },
                        Some(c) => word.push(c),
                        None => return Err(ParseError::UnclosedQuote('"')),
                    }
                }
            },
            '\\' => {
                in_word = true;
                match chars.next() {
                    Some(c) => word.push(c),
                    None => return Err(ParseError::TrailingBackslash),
                }
            },
            c => {
                in_word = true;
                word.push(c);
            },
        }
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
mod tests;