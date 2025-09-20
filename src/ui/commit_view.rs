use crate::app_state::AppState;
use crate::commit_storage;
use crate::git;
use pancurses::Input;
#[cfg(not(test))]
use pancurses::curs_set;

pub fn handle_commit_input(state: &mut AppState, input: Input, max_y: i32) {
    match input {
        Input::KeyUp => {
            state.is_commit_mode = false;
            #[cfg(not(test))]
            curs_set(0);
            state.file_cursor = state.files.len();
            state.line_cursor = 0;
            state.scroll = 0;

            if state.file_cursor < state.file_list_scroll {
                state.file_list_scroll = state.file_cursor;
            }
        }
        Input::KeyDown => {
            state.is_commit_mode = false;
            #[cfg(not(test))]
            curs_set(0);
            state.file_cursor = state.files.len() + 2;
            state.line_cursor = 0;
            state.scroll = 0;

            let file_list_height = state.main_header_height(max_y).0;
            if state.file_cursor >= state.file_list_scroll + file_list_height {
                state.file_list_scroll = state.file_cursor - file_list_height + 1;
            }
        }
        Input::Character('\t') => {
            state.is_amend_mode = !state.is_amend_mode;
            if state.is_amend_mode {
                // Switched to amend mode
                if state.amend_message.is_empty() {
                    state.amend_message =
                        git::get_previous_commit_message(&state.repo_path).unwrap_or_default();
                }
                state.commit_cursor = state.amend_message.chars().count();
            } else {
                // Switched back to commit mode
                state.commit_cursor = state.commit_message.chars().count();
            }
        }
        Input::Character('\n') => {
            if state.is_amend_mode {
                if state.amend_message.is_empty() {
                    return;
                }
                git::amend_commit(&state.repo_path, &state.amend_message)
                    .expect("Failed to amend commit.");
                let _ = commit_storage::delete_commit_message(&state.repo_path);
                state.command_history.clear();
                state.is_amend_mode = false;
            } else {
                if state.commit_message.is_empty() {
                    return;
                }
                git::commit(&state.repo_path, &state.commit_message).expect("Failed to commit.");
                let _ = commit_storage::delete_commit_message(&state.repo_path);
                state.commit_message.clear();
                state.command_history.clear();
            }

            state.amend_message =
                git::get_previous_commit_message(&state.repo_path).unwrap_or_default();

            git::add_all(&state.repo_path).expect("Failed to git add -A.");

            let staged_diff_output = git::get_staged_diff_output(&state.repo_path)
                .expect("Failed to git diff --staged.");

            if staged_diff_output.stdout.is_empty() {
                state.running = false;
            } else {
                state.refresh_diff();
                state.is_commit_mode = false;
                state.file_cursor = 0;
                #[cfg(not(test))]
                curs_set(0);
            }
        }
        Input::KeyBackspace | Input::Character('\x7f') | Input::Character('\x08') => {
            if state.commit_cursor > 0 {
                let message = if state.is_amend_mode {
                    &mut state.amend_message
                } else {
                    &mut state.commit_message
                };
                let char_index_to_remove = state.commit_cursor - 1;
                if let Some((byte_index, _)) = message.char_indices().nth(char_index_to_remove) {
                    message.remove(byte_index);
                    state.commit_cursor -= 1;
                    if !state.is_amend_mode {
                        let _ = commit_storage::save_commit_message(
                            &state.repo_path,
                            &state.commit_message,
                        );
                    }
                }
            }
        }
        Input::KeyDC => {
            let message = if state.is_amend_mode {
                &mut state.amend_message
            } else {
                &mut state.commit_message
            };
            if state.commit_cursor < message.chars().count() {
                if let Some((byte_index, _)) = message.char_indices().nth(state.commit_cursor) {
                    message.remove(byte_index);
                    if !state.is_amend_mode {
                        let _ = commit_storage::save_commit_message(
                            &state.repo_path,
                            &state.commit_message,
                        );
                    }
                }
            }
        }
        Input::KeyLeft => {
            state.commit_cursor = state.commit_cursor.saturating_sub(1);
        }
        Input::KeyRight => {
            let message_len = if state.is_amend_mode {
                state.amend_message.chars().count()
            } else {
                state.commit_message.chars().count()
            };
            state.commit_cursor = state.commit_cursor.saturating_add(1).min(message_len);
        }
        Input::Character(c) => {
            if c == '\u{1b}' {
                // ESC key
                state.is_commit_mode = false;
                state.is_amend_mode = false; // Also reset amend mode
                #[cfg(not(test))]
                curs_set(0);
            } else if c == '\u{1}' {
                // Ctrl-A: beginning of line
                state.commit_cursor = 0;
            } else if c == '\u{5}' {
                // Ctrl-E: end of line
                let message = if state.is_amend_mode {
                    &state.amend_message
                } else {
                    &state.commit_message
                };
                state.commit_cursor = message.chars().count();
            } else if c == '\u{b}' {
                // Ctrl-K: kill to end of line
                let message = if state.is_amend_mode {
                    &mut state.amend_message
                } else {
                    &mut state.commit_message
                };
                if state.commit_cursor < message.chars().count() {
                    let byte_offset = message
                        .char_indices()
                        .nth(state.commit_cursor)
                        .map_or(message.len(), |(idx, _)| idx);
                    message.truncate(byte_offset);
                    if !state.is_amend_mode {
                        let _ = commit_storage::save_commit_message(
                            &state.repo_path,
                            &state.commit_message,
                        );
                    }
                }
            } else if !c.is_control() {
                let message = if state.is_amend_mode {
                    &mut state.amend_message
                } else {
                    &mut state.commit_message
                };
                let byte_offset = message
                    .char_indices()
                    .nth(state.commit_cursor)
                    .map_or(message.len(), |(idx, _)| idx);
                message.insert(byte_offset, c);
                state.commit_cursor += 1;
                if !state.is_amend_mode {
                    let _ = commit_storage::save_commit_message(
                        &state.repo_path,
                        &state.commit_message,
                    );
                }
            }
        }
        _ => {}
    }
}
