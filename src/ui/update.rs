use crate::app_state::{AppState, Screen};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::ui::main_screen::{self, ListItem as MainScreenListItem};
use crate::ui::unstaged_screen;
use pancurses::Input;
#[cfg(not(test))]
use pancurses::curs_set;

pub fn update_state(mut state: AppState, input: Option<Input>, max_y: i32, max_x: i32) -> AppState {
    state.error_message = None;

    if let Some(input) = input {
        // Global commands
        match input {
            Input::Character('\u{3}') | Input::Character('Q') => {
                let _ = commit_storage::save_commit_message(
                    &state.repo_path,
                    &state.main_screen.commit_message,
                );
                state.running = false;
                return state;
            }
            Input::Character('<') => {
                if !state.main_screen.is_commit_mode {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.undo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff();
                    }
                    state.main_screen.is_commit_mode = state.screen == Screen::Main
                        && state.main_screen.file_cursor == state.files.len() + 1;
                    return state;
                }
            }
            Input::Character('>') => {
                if !state.main_screen.is_commit_mode {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.redo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    }
                    state.main_screen.is_commit_mode = state.screen == Screen::Main
                        && state.main_screen.file_cursor == state.files.len() + 1;
                    return state;
                }
            }
            _ => (),
        }

        match state.screen {
            Screen::Main => {
                main_screen::handle_input(&mut state, input, max_y, max_x);
            }
            Screen::Unstaged => {
                unstaged_screen::handle_input(&mut state, input, max_y);
            }
        }
    }

    state.main_screen.is_commit_mode = state.screen == Screen::Main
        && matches!(
            state.current_main_item(),
            Some(MainScreenListItem::CommitMessageInput)
        );

    state
}
