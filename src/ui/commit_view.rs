use crate::app_state::AppState;
use crate::commit_storage;
use crate::git;
use pancurses::COLOR_PAIR;
use pancurses::Input;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const COMMIT_INPUT_PREFIX: &str = " ○ ";

pub fn render_editor(
    window: &pancurses::Window,
    text: &str,
    cursor: usize,
    is_selected: bool,
    line_y: i32,
    max_x: i32,
    prefix: &str,
    scroll_offset: usize,
    scroll_extra_space: bool,
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
    let max_x_usize = if max_x < 0 { 0 } else { max_x as usize };
    let prefix_width = prefix.width();
    let available_width = max_x_usize.saturating_sub(prefix_width);

    let (displayed_text, cursor_column_relative) = build_display_line(
        text,
        cursor,
        available_width,
        scroll_offset,
        scroll_extra_space,
    );

    if !displayed_text.is_empty() {
        window.addstr(&displayed_text);
    }

    window.attroff(COLOR_PAIR(pair));

    let cursor_display_pos = prefix_width.saturating_add(cursor_column_relative);
    let clamped_cursor = cursor_display_pos.min(max_x_usize.saturating_sub(1));

    (clamped_cursor as i32, line_y)
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
        }) = state.current_main_item()
        {
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
        window.addstr(COMMIT_INPUT_PREFIX);
        let placeholder_pair = if is_selected { 16 } else { 9 };
        window.attron(COLOR_PAIR(placeholder_pair));
        window.addstr(placeholder);
        window.attroff(COLOR_PAIR(placeholder_pair));
        window.attroff(COLOR_PAIR(pair));
        (
            COMMIT_INPUT_PREFIX.width().try_into().unwrap_or_default(),
            line_y,
        )
    } else {
        render_editor(
            window,
            message,
            state.main_screen.commit_cursor,
            is_selected,
            line_y,
            max_x,
            COMMIT_INPUT_PREFIX,
            state.main_screen.commit_scroll_offset,
            state.main_screen.commit_scroll_extra_space,
        )
    }
}

pub fn compute_scroll_for_prefix(
    text: &str,
    cursor: usize,
    max_x: i32,
    prefix: &str,
) -> (usize, bool) {
    let max_x = max_x.max(0) as usize;
    let available_width = max_x.saturating_sub(prefix.width());
    if available_width == 0 || text.is_empty() {
        return (0, false);
    }
    compute_commit_scroll(text, cursor, available_width)
}

fn build_display_line(
    text: &str,
    cursor: usize,
    available_width: usize,
    scroll_offset: usize,
    scroll_extra_space: bool,
) -> (String, usize) {
    if available_width == 0 {
        return (String::new(), 0);
    }

    let chars: Vec<char> = text.chars().collect();
    let cursor_index = cursor.min(chars.len());
    let offset_index = scroll_offset.min(cursor_index).min(chars.len());
    let prefix_widths = prefix_widths_for(&chars);

    let offset_width = prefix_widths[offset_index];
    let cursor_absolute_width = prefix_widths[cursor_index];
    let ellipsis = if offset_index > 0 {
        if scroll_extra_space { "… " } else { "…" }
    } else {
        ""
    };
    let ellipsis_width = if offset_index > 0 {
        if scroll_extra_space { 2 } else { 1 }
    } else {
        0
    };

    let mut rendered = String::new();
    let mut consumed_width = 0usize;
    let mut actual_ellipsis_width = 0usize;

    if !ellipsis.is_empty() && ellipsis_width <= available_width {
        rendered.push_str(ellipsis);
        consumed_width = ellipsis_width;
        actual_ellipsis_width = ellipsis_width;
    }

    for ch in chars.iter().skip(offset_index) {
        let ch_width = ch.width().unwrap_or(0);
        if consumed_width + ch_width > available_width {
            break;
        }
        rendered.push(*ch);
        consumed_width += ch_width;
    }

    let visible_width_to_cursor = cursor_absolute_width.saturating_sub(offset_width);
    let cursor_column_relative = visible_width_to_cursor
        .saturating_add(actual_ellipsis_width)
        .min(available_width);

    (rendered, cursor_column_relative)
}

fn prefix_widths_for(chars: &[char]) -> Vec<usize> {
    let mut widths = Vec::with_capacity(chars.len() + 1);
    widths.push(0);
    for ch in chars {
        let width = ch.width().unwrap_or(0);
        let next = widths.last().copied().unwrap_or(0) + width;
        widths.push(next);
    }
    widths
}

fn compute_commit_scroll(text: &str, cursor: usize, available_width: usize) -> (usize, bool) {
    if available_width <= 4 {
        return (0, false);
    }

    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return (0, false);
    }
    let cursor_index = cursor.min(chars.len());
    if cursor_index == 0 {
        return (0, false);
    }

    let prefix_widths = prefix_widths_for(&chars);
    let cursor_absolute_width = prefix_widths[cursor_index];

    let right_scroll_trigger = available_width.saturating_sub(5);
    if cursor_absolute_width <= right_scroll_trigger {
        return (0, false);
    }

    let target_position = available_width.saturating_sub(4);
    if target_position == 0 {
        return (0, false);
    }

    let mut best_offset = cursor_index;
    let mut best_extra_space = false;
    let mut best_display_col = 0usize;
    let mut found = false;

    for &(ellipsis_width, extra_space) in &[(1usize, false), (2usize, true)] {
        if target_position < ellipsis_width || available_width < ellipsis_width {
            continue;
        }

        let required_trim = cursor_absolute_width + ellipsis_width - target_position;
        if required_trim == 0 {
            continue;
        }

        let mut offset = cursor_index;
        for (i, &width) in prefix_widths.iter().enumerate().take(cursor_index + 1) {
            if width >= required_trim {
                offset = i;
                break;
            }
        }

        let trimmed_width = prefix_widths[offset];
        let visible_width = cursor_absolute_width.saturating_sub(trimmed_width);
        let display_col = visible_width + ellipsis_width;
        if display_col > target_position {
            continue;
        }

        if !found
            || display_col > best_display_col
            || (display_col == best_display_col && offset < best_offset)
        {
            found = true;
            best_offset = offset;
            best_extra_space = extra_space;
            best_display_col = display_col;
        }

        if display_col == target_position {
            return (offset, extra_space);
        }
    }

    if found {
        return (best_offset, best_extra_space);
    }

    if available_width >= 1 && cursor_absolute_width > target_position {
        return (cursor_index, false);
    }

    (0, false)
}

fn adjust_commit_scroll_state(state: &mut AppState, text: &str, cursor: usize, max_x: i32) {
    let (offset, extra_space) = compute_scroll_for_prefix(text, cursor, max_x, COMMIT_INPUT_PREFIX);
    state.main_screen.commit_scroll_offset = offset.min(text.chars().count());
    state.main_screen.commit_scroll_extra_space = extra_space;
}

pub fn handle_generic_text_input_with_alt(text: &mut String, cursor: &mut usize, input: Input) {
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
                let new_cursor_pos =
                    if let Some(pos) = message_before_cursor.rfind(|c: char| !c.is_whitespace()) {
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

pub fn handle_generic_text_input(text: &mut String, cursor: &mut usize, input: Input) {
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

pub fn handle_commit_input_with_alt(state: &mut AppState, input: Input, max_x: i32) {
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
        let (cursor_position, message_snapshot) = {
            handle_generic_text_input_with_alt(message, cursor, input);
            (*cursor, message.clone())
        };
        adjust_commit_scroll_state(state, message_snapshot.as_str(), cursor_position, max_x);
    }
}

pub fn handle_commit_input(state: &mut AppState, input: Input, _max_y: i32, max_x: i32) {
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
                state.main_screen.commit_scroll_offset = 0;
                state.main_screen.commit_scroll_extra_space = false;
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
        let (cursor_position, message_snapshot) = {
            handle_generic_text_input(message, cursor, input);
            if !is_amend {
                let _ = commit_storage::save_commit_message(&state.repo_path, message);
            }
            (*cursor, message.clone())
        };
        adjust_commit_scroll_state(state, message_snapshot.as_str(), cursor_position, max_x);
    }
}
