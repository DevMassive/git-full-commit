use crate::app_state::AppState;
use crate::commit_storage;
use crate::git;
use pancurses::COLOR_PAIR;
use pancurses::Input;
use unicode_width::UnicodeWidthStr;

pub fn render(
    window: &pancurses::Window,
    state: &AppState,
    is_selected: bool,
    line_y: i32,
    max_x: i32,
) -> (i32, i32) {
    let pair = if is_selected { 5 } else { 1 };
    window.attron(COLOR_PAIR(pair));
    if is_selected {
        for x in 0..max_x {
            window.mvaddch(line_y, x, ' ');
        }
    }
    window.mv(line_y, 0);

    let prefix = " ○ ";
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

    window.addstr(prefix);
    if message.is_empty() {
        let pair = if is_selected { 16 } else { 9 };
        window.attron(COLOR_PAIR(pair));
        window.addstr(placeholder);
        window.attroff(COLOR_PAIR(pair));
    } else {
        window.addstr(message);
    }
    window.attroff(COLOR_PAIR(pair));

    let commit_line_y = line_y;
    let prefix_width = prefix.width();
    let message_before_cursor: String = message
        .chars()
        .take(state.main_screen.commit_cursor)
        .collect();
    let cursor_display_pos = prefix_width + message_before_cursor.width();

    let carret_x = cursor_display_pos as i32;
    let carret_y = commit_line_y;

    (carret_x, carret_y)
}

pub fn handle_commit_input(state: &mut AppState, input: Input, _max_y: i32) {
    let is_amend = matches!(
        state.current_main_item(),
        Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput { .. })
    );

    let message_to_edit = if is_amend {
        if let Some(crate::ui::main_screen::ListItem::AmendingCommitMessageInput {
            message, ..
        }) = state
            .main_screen
            .list_items
            .get_mut(state.main_screen.file_cursor)
        {
            Some(message)
        } else {
            None
        }
    } else {
        Some(&mut state.main_screen.commit_message)
    };

    let Some(message) = message_to_edit else {
        return;
    };

    match input {
        Input::Character('\n') => {
            if message.is_empty() {
                return;
            }

            if is_amend {
                if let Some(hash) = state.main_screen.amending_commit_hash.clone() {
                    let has_staged_changes = !state.files.is_empty();
                    let result = if has_staged_changes {
                        git::amend_commit_with_staged_changes(&state.repo_path, &hash, message)
                    } else {
                        git::reword_commit(&state.repo_path, &hash, message)
                    };

                    match result {
                        Ok(_) => {
                            state.command_history.clear();
                            state.main_screen.amending_commit_hash = None;
                            state.refresh_diff();
                        }
                        Err(e) => {
                            state.error_message = Some(format!("Error amending commit: {e}"));
                        }
                    }
                }
            } else {
                git::commit(&state.repo_path, message).expect("Failed to commit.");
                let _ = commit_storage::delete_commit_message(&state.repo_path);
                message.clear();
                state.command_history.clear();
            }

            git::add_all(&state.repo_path).expect("Failed to git add -A.");

            let staged_diff_output = git::get_staged_diff_output(&state.repo_path)
                .expect("Failed to git diff --staged.");

            if staged_diff_output.stdout.is_empty() {
                state.running = false;
            } else {
                state.refresh_diff();
                state.main_screen.file_cursor = 0;
            }
        }
        Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
            if state.main_screen.commit_cursor > 0 {
                let char_index_to_remove = state.main_screen.commit_cursor - 1;
                if let Some((byte_index, _)) = message.char_indices().nth(char_index_to_remove) {
                    message.remove(byte_index);
                    state.main_screen.commit_cursor -= 1;
                    if !is_amend {
                        let _ = commit_storage::save_commit_message(&state.repo_path, message);
                    }
                }
            }
        }
        Input::KeyDC => {
            if state.main_screen.commit_cursor < message.chars().count() {
                if let Some((byte_index, _)) =
                    message.char_indices().nth(state.main_screen.commit_cursor)
                {
                    message.remove(byte_index);
                    if !is_amend {
                        let _ = commit_storage::save_commit_message(&state.repo_path, message);
                    }
                }
            }
        }
        Input::KeyLeft => {
            state.main_screen.commit_cursor = state.main_screen.commit_cursor.saturating_sub(1);
        }
        Input::KeyRight => {
            let message_len = message.chars().count();
            state.main_screen.commit_cursor = state
                .main_screen
                .commit_cursor
                .saturating_add(1)
                .min(message_len);
        }
        Input::Character(c) => {
            if c == '\u{1}' {
                // Ctrl-A: beginning of line
                state.main_screen.commit_cursor = 0;
            } else if c == '\u{5}' {
                // Ctrl-E: end of line
                state.main_screen.commit_cursor = message.chars().count();
            } else if c == '\u{b}' {
                // Ctrl-K: kill to end of line
                if state.main_screen.commit_cursor < message.chars().count() {
                    let byte_offset = message
                        .char_indices()
                        .nth(state.main_screen.commit_cursor)
                        .map_or(message.len(), |(idx, _)| idx);
                    message.truncate(byte_offset);
                    if !is_amend {
                        let _ = commit_storage::save_commit_message(&state.repo_path, message);
                    }
                }
            } else if !c.is_control() {
                let byte_offset = message
                    .char_indices()
                    .nth(state.main_screen.commit_cursor)
                    .map_or(message.len(), |(idx, _)| idx);
                message.insert(byte_offset, c);
                state.main_screen.commit_cursor += 1;
                if !is_amend {
                    let _ = commit_storage::save_commit_message(&state.repo_path, message);
                }
            }
        }
        _ => {}
    }
}
