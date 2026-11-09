use super::*;
use alloc::string::ToString;

fn editor() -> LineEditor {
    LineEditor::new(40)
}

fn type_str(editor: &mut LineEditor, s: &str) {
    for c in s.chars() {
        assert_eq!(editor.handle(Key::Char(c)), Event::Continue);
    }
}

fn press(editor: &mut LineEditor, keys: &[Key]) {
    for &key in keys {
        assert_eq!(editor.handle(key), Event::Continue);
    }
}

fn submit(editor: &mut LineEditor, s: &str) {
    type_str(editor, s);
    assert_eq!(editor.handle(Key::Enter), Event::Submit(s.to_string()));
}

fn rendered(editor: &LineEditor, prompt: &str) -> String {
    let mut out = String::new();
    editor.render(prompt, &mut out).unwrap();
    out
}

// ввод и курсор

#[test_case]
fn test_new_editor_is_empty() {
    let editor = editor();
    assert_eq!(editor.line(), "");
    assert_eq!(editor.cursor(), 0);
    assert!(editor.history().is_empty());
}

#[test_case]
fn test_typing_appends() {
    let mut editor = editor();
    type_str(&mut editor, "ls -l");
    assert_eq!(editor.line(), "ls -l");
    assert_eq!(editor.cursor(), 5);
}

#[test_case]
fn test_unicode_counts_as_one_char() {
    let mut editor = editor();
    type_str(&mut editor, "жё");
    assert_eq!(editor.cursor(), 2);
    press(&mut editor, &[Key::Backspace]);
    assert_eq!(editor.line(), "ж");
}

#[test_case]
fn test_insert_in_middle() {
    let mut editor = editor();
    type_str(&mut editor, "helo");
    press(&mut editor, &[Key::Left]);
    type_str(&mut editor, "l");
    assert_eq!(editor.line(), "hello");
    assert_eq!(editor.cursor(), 4);
}

#[test_case]
fn test_left_stops_at_start() {
    let mut editor = editor();
    type_str(&mut editor, "ab");
    press(&mut editor, &[Key::Left, Key::Left, Key::Left]);
    assert_eq!(editor.cursor(), 0);
}

#[test_case]
fn test_right_stops_at_end() {
    let mut editor = editor();
    type_str(&mut editor, "ab");
    press(&mut editor, &[Key::Left, Key::Right, Key::Right]);
    assert_eq!(editor.cursor(), 2);
}

#[test_case]
fn test_home_and_end() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    press(&mut editor, &[Key::Home]);
    assert_eq!(editor.cursor(), 0);
    press(&mut editor, &[Key::End]);
    assert_eq!(editor.cursor(), 3);
}

#[test_case]
fn test_ctrl_a_and_ctrl_e() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    press(&mut editor, &[Key::Ctrl('a')]);
    assert_eq!(editor.cursor(), 0);
    press(&mut editor, &[Key::Ctrl('e')]);
    assert_eq!(editor.cursor(), 3);
}

#[test_case]
fn test_max_len_limits_input() {
    let mut editor = LineEditor::new(3);
    type_str(&mut editor, "abcdef");
    assert_eq!(editor.line(), "abc");
    assert_eq!(editor.cursor(), 3);
}

#[test_case]
fn test_set_max_len() {
    let mut editor = LineEditor::new(0);
    type_str(&mut editor, "a");
    assert_eq!(editor.line(), "");
    editor.set_max_len(2);
    type_str(&mut editor, "abc");
    assert_eq!(editor.line(), "ab");
}

#[test_case]
fn test_tab_and_unknown_ctrl_ignored() {
    let mut editor = editor();
    type_str(&mut editor, "a");
    press(&mut editor, &[Key::Tab, Key::Ctrl('z')]);
    assert_eq!(editor.line(), "a");
}

// удаление

#[test_case]
fn test_backspace_deletes_before_cursor() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    press(&mut editor, &[Key::Left, Key::Backspace]);
    assert_eq!(editor.line(), "ac");
    assert_eq!(editor.cursor(), 1);
}

#[test_case]
fn test_backspace_at_start_does_nothing() {
    let mut editor = editor();
    type_str(&mut editor, "ab");
    press(&mut editor, &[Key::Home, Key::Backspace]);
    assert_eq!(editor.line(), "ab");
    assert_eq!(editor.cursor(), 0);
}

#[test_case]
fn test_delete_deletes_under_cursor() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    press(&mut editor, &[Key::Home, Key::Delete]);
    assert_eq!(editor.line(), "bc");
    assert_eq!(editor.cursor(), 0);
}

#[test_case]
fn test_delete_at_end_does_nothing() {
    let mut editor = editor();
    type_str(&mut editor, "ab");
    press(&mut editor, &[Key::Delete]);
    assert_eq!(editor.line(), "ab");
}

#[test_case]
fn test_ctrl_u_deletes_before_cursor() {
    let mut editor = editor();
    type_str(&mut editor, "rm file");
    for _ in 0..4 {
        press(&mut editor, &[Key::Left]);
    }
    press(&mut editor, &[Key::Ctrl('u')]);
    assert_eq!(editor.line(), "file");
    assert_eq!(editor.cursor(), 0);
}

// Enter, Ctrl+C, Ctrl+D, Ctrl+L

#[test_case]
fn test_enter_submits_and_clears() {
    let mut editor = editor();
    submit(&mut editor, "echo hi");
    assert_eq!(editor.line(), "");
    assert_eq!(editor.cursor(), 0);
}

#[test_case]
fn test_enter_with_cursor_in_middle_submits_whole_line() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    press(&mut editor, &[Key::Home]);
    assert_eq!(editor.handle(Key::Enter), Event::Submit("abc".to_string()));
}

#[test_case]
fn test_enter_on_empty_line() {
    let mut editor = editor();
    assert_eq!(editor.handle(Key::Enter), Event::Submit(String::new()));
}

#[test_case]
fn test_ctrl_c_interrupts_with_cursor_at_end() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    press(&mut editor, &[Key::Home]);
    assert_eq!(editor.handle(Key::Ctrl('c')), Event::Interrupt);
    // строка ещё видна, чтобы после неё напечатать ^C
    assert_eq!(editor.line(), "abc");
    assert_eq!(editor.cursor(), 3);
    editor.clear();
    assert_eq!(editor.line(), "");
}

#[test_case]
fn test_interrupted_line_not_in_history() {
    let mut editor = editor();
    type_str(&mut editor, "abc");
    editor.handle(Key::Ctrl('c'));
    editor.clear();
    assert!(editor.history().is_empty());
}

#[test_case]
fn test_ctrl_d_on_empty_line_is_eof() {
    let mut editor = editor();
    assert_eq!(editor.handle(Key::Ctrl('d')), Event::Eof);
}

#[test_case]
fn test_ctrl_d_on_non_empty_line_deletes() {
    let mut editor = editor();
    type_str(&mut editor, "ab");
    press(&mut editor, &[Key::Home, Key::Ctrl('d')]);
    assert_eq!(editor.line(), "b");
}

#[test_case]
fn test_ctrl_l_clears_screen_keeps_line() {
    let mut editor = editor();
    type_str(&mut editor, "ab");
    assert_eq!(editor.handle(Key::Ctrl('l')), Event::ClearScreen);
    assert_eq!(editor.line(), "ab");
}

// история

#[test_case]
fn test_submitted_lines_go_to_history() {
    let mut editor = editor();
    submit(&mut editor, "one");
    submit(&mut editor, "two");
    assert_eq!(editor.history(), &["one".to_string(), "two".to_string()]);
}

#[test_case]
fn test_empty_and_blank_lines_not_in_history() {
    let mut editor = editor();
    submit(&mut editor, "");
    submit(&mut editor, "   ");
    assert!(editor.history().is_empty());
}

#[test_case]
fn test_repeated_line_stored_once() {
    let mut editor = editor();
    submit(&mut editor, "ls");
    submit(&mut editor, "ls");
    submit(&mut editor, "pwd");
    submit(&mut editor, "ls");
    assert_eq!(
        editor.history(),
        &["ls".to_string(), "pwd".to_string(), "ls".to_string()]
    );
}

#[test_case]
fn test_up_loads_previous_lines() {
    let mut editor = editor();
    submit(&mut editor, "one");
    submit(&mut editor, "two");
    press(&mut editor, &[Key::Up]);
    assert_eq!(editor.line(), "two");
    assert_eq!(editor.cursor(), 3);
    press(&mut editor, &[Key::Up]);
    assert_eq!(editor.line(), "one");
}

#[test_case]
fn test_up_stops_at_oldest() {
    let mut editor = editor();
    submit(&mut editor, "one");
    press(&mut editor, &[Key::Up, Key::Up, Key::Up]);
    assert_eq!(editor.line(), "one");
}

#[test_case]
fn test_up_with_empty_history_does_nothing() {
    let mut editor = editor();
    type_str(&mut editor, "draft");
    press(&mut editor, &[Key::Up]);
    assert_eq!(editor.line(), "draft");
}

#[test_case]
fn test_down_returns_to_draft() {
    let mut editor = editor();
    submit(&mut editor, "one");
    submit(&mut editor, "two");
    type_str(&mut editor, "dra");
    press(&mut editor, &[Key::Up, Key::Up, Key::Down]);
    assert_eq!(editor.line(), "two");
    press(&mut editor, &[Key::Down]);
    assert_eq!(editor.line(), "dra");
    assert_eq!(editor.cursor(), 3);
}

#[test_case]
fn test_down_without_history_navigation_does_nothing() {
    let mut editor = editor();
    submit(&mut editor, "one");
    type_str(&mut editor, "x");
    press(&mut editor, &[Key::Down]);
    assert_eq!(editor.line(), "x");
}

#[test_case]
fn test_edited_history_line_submits_edit() {
    let mut editor = editor();
    submit(&mut editor, "ls");
    press(&mut editor, &[Key::Up]);
    type_str(&mut editor, " -l");
    assert_eq!(
        editor.handle(Key::Enter),
        Event::Submit("ls -l".to_string())
    );
    assert_eq!(editor.history(), &["ls".to_string(), "ls -l".to_string()]);
}

#[test_case]
fn test_submit_resets_history_position() {
    let mut editor = editor();
    submit(&mut editor, "one");
    submit(&mut editor, "two");
    press(&mut editor, &[Key::Up, Key::Up]);
    editor.handle(Key::Enter);
    // после Enter листаем снова с самой новой строки
    press(&mut editor, &[Key::Up]);
    assert_eq!(editor.line(), "one");
    press(&mut editor, &[Key::Up]);
    assert_eq!(editor.line(), "two");
}

#[test_case]
fn test_history_line_truncated_to_max_len() {
    let mut editor = LineEditor::new(40);
    submit(&mut editor, "a long command");
    editor.set_max_len(6);
    press(&mut editor, &[Key::Up]);
    assert_eq!(editor.line(), "a long");
}

// вывод

#[test_case]
fn test_render_empty_line() {
    let editor = editor();
    assert_eq!(rendered(&editor, "> "), "\r> \x1b[K");
}

#[test_case]
fn test_render_line_with_cursor_at_end() {
    let mut editor = editor();
    type_str(&mut editor, "ls");
    assert_eq!(rendered(&editor, "$ "), "\r$ ls\x1b[K");
}

#[test_case]
fn test_render_moves_cursor_back() {
    let mut editor = editor();
    type_str(&mut editor, "hello");
    press(&mut editor, &[Key::Left, Key::Left]);
    assert_eq!(rendered(&editor, "> "), "\r> hello\x1b[K\x1b[2D");
}

#[test_case]
fn test_render_unicode() {
    let mut editor = editor();
    type_str(&mut editor, "жж");
    press(&mut editor, &[Key::Left]);
    // курсор двигается на символы, а не на байты
    assert_eq!(rendered(&editor, "> "), "\r> жж\x1b[K\x1b[1D");
}
