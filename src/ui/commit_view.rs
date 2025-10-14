use crate::app_state::AppState;
use crate::commit_storage;
use crate::git;
use pancurses::COLOR_PAIR;
use pancurses::Input;
use unicode_width::UnicodeWidthStr;

pub fn render_editor(
    window: &pancurses::Window,
    text: &str,
    cursor: usize,
    is_selected: bool,
    line_y: i32,
    max_x: i32,
    prefix: &str,
) -> (i32, i32) {
    let pair = if is_selected { 5 } else { 1 };
    window.attron(COLOR_PAIR(pair));
    if is_selected {
        for x in 0..max_x {
            window.mvaddch(line_y, x, ' ');
        }
    }
    window.mv(line_y, 0);

    window.addstr(prefix);
    let available_width = (max_x as usize).saturating_sub(prefix.width());
    let mut truncated_message = String::new();
    let mut current_width = 0;
    for ch in text.chars() {
        let char_width = ch.to_string().width();
        if current_width + char_width > available_width {
            break;
        }
        truncated_message.push(ch);
        current_width += char_width;
    }
    window.addstr(&truncated_message);

    window.attroff(COLOR_PAIR(pair));

    let commit_line_y = line_y;
    let prefix_width = prefix.width();
    let message_before_cursor: String = text.chars().take(cursor).collect();
    let cursor_display_pos = prefix_width + message_before_cursor.width();

    let carret_x = cursor_display_pos as i32;
    let carret_y = commit_line_y;

    (carret_x, carret_y)
}

pub fn render(
    window: &pancurses::Window,
    state: &AppState,
    is_selected: bool,
    line_y: i32,
    max_x: i32,
) -> (i32, i32) {
    let (message, placeholder) =
        if let Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput {
            message, ..
        }) = state.current_main_item() {
            (message.as_str(), "Enter amend message...")
        } else {
            (
                state.main_screen.commit_message.as_str(),
                "Enter commit message...",
            )
        };

    if message.is_empty() {
        let pair = if is_selected { 5 } else { 1 };
        window.attron(COLOR_PAIR(pair));
        if is_selected {
            for x in 0..max_x {
                window.mvaddch(line_y, x, ' ');
            }
        }
        window.mv(line_y, 0);
        let prefix = " ○ ";
        window.addstr(prefix);
        let placeholder_pair = if is_selected { 16 } else { 9 };
        window.attron(COLOR_PAIR(placeholder_pair));
        window.addstr(placeholder);
        window.attroff(COLOR_PAIR(placeholder_pair));
        window.attroff(COLOR_PAIR(pair));
        (0, 0)
    } else {
        render_editor(
            window,
            message,
            state.main_screen.commit_cursor,
            is_selected,
            line_y,
            max_x,
            " ○ ",
        )
    }
}

pub fn handle_generic_text_input_with_alt(
    text: &mut String,
    cursor: &mut usize,
    input: Input,
) {
    match input {
        Input::KeyLeft | Input::Character('b') => {
            let message_chars: Vec<char> = text.chars().collect();
            let mut i = cursor.saturating_sub(1);
            while i > 0 && message_chars.get(i).is_some_and(|c| c.is_whitespace()) {
                i -= 1;
            }
            while i > 0 && message_chars.get(i).is_some_and(|c| !c.is_whitespace()) {
                i -= 1;
            }
            if i > 0 && message_chars.get(i).is_some_and(|c| c.is_whitespace()) {
                i += 1;
            }
            *cursor = i;
        }
        Input::KeyRight | Input::Character('f') => {
            let message_chars: Vec<char> = text.chars().collect();
            let len = message_chars.len();
            let mut i = *cursor;
            while i < len && message_chars.get(i).is_some_and(|c| !c.is_whitespace()) {
                i += 1;
            }
            while i < len && message_chars.get(i).is_some_and(|c| c.is_whitespace()) {
                i += 1;
            }
            *cursor = i;
        }
        Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
            let cursor_pos = *cursor;
            if cursor_pos > 0 {
                let message_before_cursor: String = text.chars().take(cursor_pos).collect();
                let new_cursor_pos = if let Some(pos) =
                    message_before_cursor.rfind(|c: char| !c.is_whitespace())
                {
                    if let Some(pos) = message_before_cursor[..pos].rfind(char::is_whitespace) {
                        pos + 1
                    } else {
                        0
                    }
                } else {
                    0
                };

                let start_byte = text
                    .char_indices()
                    .nth(new_cursor_pos)
                    .map_or(0, |(idx, _)| idx);
                let end_byte = text
                    .char_indices()
                    .nth(cursor_pos)
                    .map_or(text.len(), |(idx, _)| idx);

                text.replace_range(start_byte..end_byte, "");
                *cursor = new_cursor_pos;
            }
        }
        _ => {}
    }
}

pub fn handle_generic_text_input(
    text: &mut String,
    cursor: &mut usize,
    input: Input,
) {
    match input {
        Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
            if *cursor > 0 {
                let char_index_to_remove = *cursor - 1;
                if let Some((byte_index, _)) = text.char_indices().nth(char_index_to_remove) {
                    text.remove(byte_index);
                    *cursor -= 1;
                }
            }
        }
        Input::KeyDC => {
            if *cursor < text.chars().count() {
                if let Some((byte_index, _)) = text.char_indices().nth(*cursor) {
                    text.remove(byte_index);
                }
            }
        }
        Input::KeyLeft => {
            *cursor = cursor.saturating_sub(1);
        }
        Input::KeyRight => {
            let message_len = text.chars().count();
            *cursor = cursor.saturating_add(1).min(message_len);
        }
        Input::Character(c) => {
            if c == '\u{1}' {
                // Ctrl-A: beginning of line
                *cursor = 0;
            } else if c == '\u{5}' {
                // Ctrl-E: end of line
                *cursor = text.chars().count();
            } else if c == '\u{b}' {
                // Ctrl-K: kill to end of line
                if *cursor < text.chars().count() {
                    let byte_offset = text
                        .char_indices()
                        .nth(*cursor)
                        .map_or(text.len(), |(idx, _)| idx);
                    text.truncate(byte_offset);
                }
            } else if !c.is_control() {
                let byte_offset = text
                    .char_indices()
                    .nth(*cursor)
                    .map_or(text.len(), |(idx, _)| idx);
                text.insert(byte_offset, c);
                *cursor += 1;
            }
        }
        _ => {}
    }
}

pub fn handle_commit_input_with_alt(state: &mut AppState, input: Input) {
    let is_amend = matches!(
        state.current_main_item(),
        Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput { .. })
    );

    let (message_to_edit, cursor_to_edit) = if is_amend {
        if let Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput {
            message, ..
        }) = state
            .main_screen
            .list_items
            .get_mut(state.main_screen.file_cursor)
        {
            (Some(message), Some(&mut state.main_screen.commit_cursor))
        } else {
            (None, None)
        }
    } else {
        (
            Some(&mut state.main_screen.commit_message),
            Some(&mut state.main_screen.commit_cursor),
        )
    };

    if let (Some(message), Some(cursor)) = (message_to_edit, cursor_to_edit) {
        handle_generic_text_input_with_alt(message, cursor, input);
    }
}

pub fn handle_commit_input(state: &mut AppState, input: Input, _max_y: i32) {
    let is_amend = matches!(
        state.current_main_item(),
        Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput { .. })
    );

    let (message_to_edit, cursor_to_edit) = if is_amend {
        if let Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput {
            message, ..
        }) = state
            .main_screen
            .list_items
            .get_mut(state.main_screen.file_cursor)
        {
            (Some(message), Some(&mut state.main_screen.commit_cursor))
        } else {
            (None, None)
        }
    } else {
        (
            Some(&mut state.main_screen.commit_message),
            Some(&mut state.main_screen.commit_cursor),
        )
    };

    let (Some(message), Some(cursor)) = (message_to_edit, cursor_to_edit) else {
        return;
    };

    if input == Input::Character('\n') {
        if message.is_empty() {
            return;
        }

        let commit_result = if is_amend {
            if let Some(hash) = state.main_screen.amending_commit_hash.clone() {
                let has_staged_changes = !state.files.is_empty();
                let result = if has_staged_changes {
                    git::amend_commit_with_staged_changes(&state.repo_path, &hash, message)
                } else {
                    git::reword_commit(&state.repo_path, &hash, message)
                };
                state.main_screen.amending_commit_hash = None;
                result
            } else {
                // This case should not happen, but for safety...
                return;
            }
        } else {
            let result = git::commit(&state.repo_path, message);
            if result.is_ok() {
                let _ = commit_storage::delete_commit_message(&state.repo_path);
                message.clear();
            }
            result
        };

        if let Err(e) = commit_result {
            state.error_message = Some(format!("Error committing: {e}"));
            return;
        }

        state.command_history.clear();
        git::add_all(&state.repo_path).expect("Failed to git add -A.");

        let staged_diff_output =
            git::get_staged_diff_output(&state.repo_path).expect("Failed to git diff --staged.");

        if staged_diff_output.stdout.is_empty() {
            state.running = false;
        } else {
            state.refresh_diff(true);
        }
    } else {
        handle_generic_text_input(message, cursor, input);
        if !is_amend {
            let _ = commit_storage::save_commit_message(&state.repo_path, message);
        }
    }
}