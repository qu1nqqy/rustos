use super::*;
use alloc::string::ToString;

fn words(line: &str) -> Vec<String> {
    split(line).expect("line should parse")
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[test_case]
fn test_empty_line() {
    assert!(words("").is_empty());
}

#[test_case]
fn test_only_spaces() {
    assert!(words("   \t  ").is_empty());
}

#[test_case]
fn test_single_word() {
    assert_eq!(words("ls"), strings(&["ls"]));
}

#[test_case]
fn test_words_separated_by_spaces() {
    assert_eq!(words("ls -l /etc"), strings(&["ls", "-l", "/etc"]));
}

#[test_case]
fn test_repeated_and_edge_whitespace() {
    assert_eq!(words("  cp \t a    b  "), strings(&["cp", "a", "b"]));
}

#[test_case]
fn test_single_quotes_keep_spaces() {
    assert_eq!(words("echo 'a  b'"), strings(&["echo", "a  b"]));
}

#[test_case]
fn test_single_quotes_keep_backslash_and_double_quote() {
    assert_eq!(words(r#"echo 'a\b"c'"#), strings(&["echo", r#"a\b"c"#]));
}

#[test_case]
fn test_double_quotes_keep_spaces() {
    assert_eq!(words(r#"echo "a  b""#), strings(&["echo", "a  b"]));
}

#[test_case]
fn test_double_quotes_escape_quote_and_backslash() {
    assert_eq!(words(r#"echo "a\"b\\c""#), strings(&["echo", r#"a"b\c"#]));
}

#[test_case]
fn test_double_quotes_keep_other_backslashes() {
    assert_eq!(words(r#"echo "a\nb""#), strings(&["echo", r"a\nb"]));
}

#[test_case]
fn test_double_quotes_keep_single_quote() {
    assert_eq!(words(r#"echo "it's""#), strings(&["echo", "it's"]));
}

#[test_case]
fn test_backslash_escapes_space() {
    assert_eq!(words(r"cat my\ file"), strings(&["cat", "my file"]));
}

#[test_case]
fn test_backslash_escapes_quote() {
    assert_eq!(words(r#"echo \"hi\""#), strings(&["echo", r#""hi""#]));
}

#[test_case]
fn test_quotes_join_with_word() {
    assert_eq!(
        words(r#"echo a"b c"d'e f'"#),
        strings(&["echo", "ab cde f"])
    );
}

#[test_case]
fn test_empty_quotes_give_empty_word() {
    assert_eq!(words(r#"echo "" ''"#), strings(&["echo", "", ""]));
}

#[test_case]
fn test_unicode_words() {
    assert_eq!(
        words("echo привет мир"),
        strings(&["echo", "привет", "мир"])
    );
}

#[test_case]
fn test_unclosed_single_quote() {
    assert_eq!(split("echo 'abc"), Err(ParseError::UnclosedQuote('\'')));
}

#[test_case]
fn test_unclosed_double_quote() {
    assert_eq!(split(r#"echo "abc"#), Err(ParseError::UnclosedQuote('"')));
}

#[test_case]
fn test_unclosed_double_quote_after_backslash() {
    assert_eq!(split(r#"echo "abc\"#), Err(ParseError::UnclosedQuote('"')));
}

#[test_case]
fn test_trailing_backslash() {
    assert_eq!(split(r"echo abc\"), Err(ParseError::TrailingBackslash));
}

#[test_case]
fn test_error_messages() {
    assert_eq!(
        ParseError::UnclosedQuote('"').to_string(),
        "unclosed quote \""
    );
    assert_eq!(
        ParseError::TrailingBackslash.to_string(),
        "trailing backslash"
    );
}
