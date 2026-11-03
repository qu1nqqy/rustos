use super::*;

/// Байты по очереди, в результате - только получившиеся клавиши
fn decode(bytes: &[u8]) -> ([Option<Key>; 16], usize) {
    let mut decoder = SerialDecoder::new();
    let mut keys = [None; 16];
    let mut len = 0;
    for &byte in bytes {
        if let Some(key) = decoder.feed(byte) {
            keys[len] = Some(key);
            len += 1;
        }
    }
    (keys, len)
}

fn assert_keys(bytes: &[u8], expected: &[Key]) {
    let (keys, len) = decode(bytes);
    assert_eq!(len, expected.len(), "keys: {:?}", &keys[..len]);
    for (key, expected) in keys[..len].iter().zip(expected) {
        assert_eq!(key.as_ref(), Some(expected));
    }
}

#[test_case]
fn test_printable_ascii() {
    assert_keys(
        b"ls -l",
        &[
            Key::Char('l'),
            Key::Char('s'),
            Key::Char(' '),
            Key::Char('-'),
            Key::Char('l'),
        ],
    );
}

#[test_case]
fn test_carriage_return_is_enter() {
    // в «сыром» режиме терминал присылает Enter как '\r'
    assert_keys(b"\r", &[Key::Enter]);
}

#[test_case]
fn test_line_feed_is_enter() {
    assert_keys(b"\n", &[Key::Enter]);
}

#[test_case]
fn test_crlf_is_one_enter() {
    assert_keys(b"a\r\nb", &[Key::Char('a'), Key::Enter, Key::Char('b')]);
}

#[test_case]
fn test_two_line_feeds_are_two_enters() {
    assert_keys(b"\n\n", &[Key::Enter, Key::Enter]);
}

#[test_case]
fn test_two_carriage_returns_are_two_enters() {
    assert_keys(b"\r\r", &[Key::Enter, Key::Enter]);
}

#[test_case]
fn test_backspace() {
    assert_keys(&[0x7f, 0x08], &[Key::Backspace, Key::Backspace]);
}

#[test_case]
fn test_tab() {
    assert_keys(b"\t", &[Key::Tab]);
}

#[test_case]
fn test_ctrl_keys() {
    assert_keys(
        &[0x03, 0x04, 0x0c],
        &[Key::Ctrl('c'), Key::Ctrl('d'), Key::Ctrl('l')],
    );
}

#[test_case]
fn test_arrows() {
    assert_keys(
        b"\x1b[A\x1b[B\x1b[C\x1b[D",
        &[Key::Up, Key::Down, Key::Right, Key::Left],
    );
}

#[test_case]
fn test_home_end_csi() {
    assert_keys(b"\x1b[H\x1b[F", &[Key::Home, Key::End]);
}

#[test_case]
fn test_home_end_delete_tilde() {
    assert_keys(
        b"\x1b[1~\x1b[4~\x1b[3~\x1b[7~\x1b[8~",
        &[Key::Home, Key::End, Key::Delete, Key::Home, Key::End],
    );
}

#[test_case]
fn test_ss3_keys() {
    // в режиме «приложения» терминал присылает ESC O вместо ESC [
    assert_keys(b"\x1bOA\x1bOH\x1bOF", &[Key::Up, Key::Home, Key::End]);
}

#[test_case]
fn test_arrow_with_modifier() {
    // Ctrl + стрелка вправо
    assert_keys(b"\x1b[1;5C", &[Key::Right]);
}

#[test_case]
fn test_unknown_sequences_ignored() {
    // Shift+Tab и F5: не поддерживаем, но и не ломаемся
    assert_keys(b"\x1b[Za\x1b[15~b", &[Key::Char('a'), Key::Char('b')]);
}

#[test_case]
fn test_unknown_tilde_param_ignored() {
    assert_keys(b"\x1b[2~x", &[Key::Char('x')]);
}

#[test_case]
fn test_alt_key_ignored() {
    // Alt + x приходит как ESC x
    assert_keys(b"\x1bxy", &[Key::Char('y')]);
}

#[test_case]
fn test_broken_csi_recovers() {
    // управляющий байт посреди последовательности
    assert_keys(b"\x1b[\x01a", &[Key::Char('a')]);
}

#[test_case]
fn test_huge_csi_param_does_not_overflow() {
    assert_keys(b"\x1b[99999999~a", &[Key::Char('a')]);
}

#[test_case]
fn test_utf8_two_bytes() {
    assert_keys("жё".as_bytes(), &[Key::Char('ж'), Key::Char('ё')]);
}

#[test_case]
fn test_utf8_three_bytes() {
    assert_keys("€".as_bytes(), &[Key::Char('€')]);
}

#[test_case]
fn test_utf8_four_bytes() {
    assert_keys("🦀".as_bytes(), &[Key::Char('🦀')]);
}

#[test_case]
fn test_utf8_mixed_with_ascii() {
    assert_keys(
        "aжb".as_bytes(),
        &[Key::Char('a'), Key::Char('ж'), Key::Char('b')],
    );
}

#[test_case]
fn test_utf8_interrupted_by_ascii() {
    // первый байт 'ж', а за ним сразу 'a': обрывок выбрасывается, 'a' остаётся
    assert_keys(&[0xd0, b'a'], &[Key::Char('a')]);
}

#[test_case]
fn test_stray_continuation_byte_ignored() {
    assert_keys(&[0xb6, b'a'], &[Key::Char('a')]);
}

#[test_case]
fn test_invalid_utf8_bytes_ignored() {
    // 0xc0, 0xc1 и 0xf5..=0xff в UTF-8 не встречаются
    assert_keys(&[0xc0, 0xff, b'a'], &[Key::Char('a')]);
}

#[test_case]
fn test_overlong_utf8_ignored() {
    // E0 80 80 - «слишком длинная» запись нуля, from_utf8 её отвергает
    assert_keys(&[0xe0, 0x80, 0x80, b'a'], &[Key::Char('a')]);
}

#[test_case]
fn test_decoder_returns_none_inside_sequence() {
    let mut decoder = SerialDecoder::new();
    assert_eq!(decoder.feed(0x1b), None);
    assert_eq!(decoder.feed(b'['), None);
    assert_eq!(decoder.feed(b'A'), Some(Key::Up));
}
