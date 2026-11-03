use super::*;

#[test_case]
fn test_enter_codes() {
    assert_eq!(Key::from_control('\n'), Some(Key::Enter));
    assert_eq!(Key::from_control('\r'), Some(Key::Enter));
}

#[test_case]
fn test_backspace_codes() {
    // клавиатура отдаёт BS (0x08), терминалы обычно DEL (0x7f)
    assert_eq!(Key::from_control('\u{8}'), Some(Key::Backspace));
    assert_eq!(Key::from_control('\u{7f}'), Some(Key::Backspace));
}

#[test_case]
fn test_tab_code() {
    assert_eq!(Key::from_control('\t'), Some(Key::Tab));
}

#[test_case]
fn test_ctrl_letters() {
    assert_eq!(Key::from_control('\u{1}'), Some(Key::Ctrl('a')));
    assert_eq!(Key::from_control('\u{3}'), Some(Key::Ctrl('c')));
    assert_eq!(Key::from_control('\u{4}'), Some(Key::Ctrl('d')));
    assert_eq!(Key::from_control('\u{c}'), Some(Key::Ctrl('l')));
    assert_eq!(Key::from_control('\u{1a}'), Some(Key::Ctrl('z')));
}

#[test_case]
fn test_other_control_codes_ignored() {
    assert_eq!(Key::from_control('\u{0}'), None);
    assert_eq!(Key::from_control('\u{1b}'), None);
    assert_eq!(Key::from_control('\u{1f}'), None);
}

#[test_case]
fn test_printable_chars_are_not_control() {
    assert_eq!(Key::from_control('a'), None);
    assert_eq!(Key::from_control(' '), None);
}
