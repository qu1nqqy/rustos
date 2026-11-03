use super::*;

// скан-коды набора 1: отпускание клавиши = код нажатия | 0x80,
// у «серых» клавиш (стрелки, Home, End, Delete) перед кодом идёт 0xE0
const EXTENDED: u8 = 0xe0;
const A: u8 = 0x1e;
const C: u8 = 0x2e;
const ONE: u8 = 0x02;
const SPACE: u8 = 0x39;
const ENTER: u8 = 0x1c;
const BACKSPACE: u8 = 0x0e;
const TAB: u8 = 0x0f;
const ESCAPE: u8 = 0x01;
const LSHIFT: u8 = 0x2a;
const LCTRL: u8 = 0x1d;
const UP: u8 = 0x48;
const DOWN: u8 = 0x50;
const LEFT: u8 = 0x4b;
const RIGHT: u8 = 0x4d;
const HOME: u8 = 0x47;
const END: u8 = 0x4f;
const DELETE: u8 = 0x53;

const fn release(scancode: u8) -> u8 {
    scancode | 0x80
}

/// Скан-коды по очереди, в результате - только получившиеся клавиши
fn feed_all(decoder: &mut KeyboardDecoder, scancodes: &[u8]) -> ([Option<Key>; 8], usize) {
    let mut keys = [None; 8];
    let mut len = 0;
    for &scancode in scancodes {
        if let Some(key) = decoder.feed(scancode) {
            keys[len] = Some(key);
            len += 1;
        }
    }
    (keys, len)
}

fn assert_keys(scancodes: &[u8], expected: &[Key]) {
    let mut decoder = KeyboardDecoder::new();
    let (keys, len) = feed_all(&mut decoder, scancodes);
    assert_eq!(len, expected.len());
    for (key, expected) in keys[..len].iter().zip(expected) {
        assert_eq!(key.as_ref(), Some(expected));
    }
}

#[test_case]
fn test_letter() {
    assert_keys(&[A], &[Key::Char('a')]);
}

#[test_case]
fn test_release_gives_nothing() {
    assert_keys(&[A, release(A)], &[Key::Char('a')]);
}

#[test_case]
fn test_digit_and_space() {
    assert_keys(&[ONE, SPACE], &[Key::Char('1'), Key::Char(' ')]);
}

#[test_case]
fn test_shift_gives_uppercase() {
    assert_keys(
        &[LSHIFT, A, release(A), release(LSHIFT), A],
        &[Key::Char('A'), Key::Char('a')],
    );
}

#[test_case]
fn test_shift_gives_symbol() {
    assert_keys(&[LSHIFT, ONE], &[Key::Char('!')]);
}

#[test_case]
fn test_enter_backspace_tab() {
    assert_keys(
        &[ENTER, BACKSPACE, TAB],
        &[Key::Enter, Key::Backspace, Key::Tab],
    );
}

#[test_case]
fn test_escape_ignored() {
    assert_keys(&[ESCAPE, A], &[Key::Char('a')]);
}

#[test_case]
fn test_ctrl_letter() {
    assert_keys(
        &[LCTRL, C, release(C), release(LCTRL), C],
        &[Key::Ctrl('c'), Key::Char('c')],
    );
}

#[test_case]
fn test_arrows() {
    assert_keys(
        &[
            EXTENDED, UP, EXTENDED, DOWN, EXTENDED, LEFT, EXTENDED, RIGHT,
        ],
        &[Key::Up, Key::Down, Key::Left, Key::Right],
    );
}

#[test_case]
fn test_home_end_delete() {
    assert_keys(
        &[EXTENDED, HOME, EXTENDED, END, EXTENDED, DELETE],
        &[Key::Home, Key::End, Key::Delete],
    );
}

#[test_case]
fn test_extended_prefix_alone_gives_nothing() {
    let mut decoder = KeyboardDecoder::new();
    assert_eq!(decoder.feed(EXTENDED), None);
    assert_eq!(decoder.feed(LEFT), Some(Key::Left));
}

#[test_case]
fn test_shift_alone_gives_nothing() {
    assert_keys(&[LSHIFT, release(LSHIFT)], &[]);
}
