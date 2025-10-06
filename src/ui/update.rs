use crate::app_state::{AppState, FocusedPane};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::ui::main_screen::{self, ListItem as MainScreenListItem};
use pancurses::Input;

pub fn update_state(mut state: AppState, input: Option<Input>, max_y: i32, max_x: i32) -> AppState {
    state.error_message = None;

    if let Some(input) = input {
        // Global commands
        match input {
            Input::Character('\t') => {
                if !state.is_in_input_mode() {
                    let current_file_path = match state.focused_pane {
                        FocusedPane::Main => state.current_main_file().map(|f| f.file_name.clone()),
                        FocusedPane::Unstaged => state.get_unstaged_file().map(|f| f.file_name.clone()),
                    };

                    state.focused_pane = match state.focused_pane {
                        FocusedPane::Main => FocusedPane::Unstaged,
                        FocusedPane::Unstaged => FocusedPane::Main,
                    };

                    if let Some(path) = current_file_path {
                        match state.focused_pane {
                            FocusedPane::Main => {
                                if let Some(index) = state.main_screen.list_items.iter().position(|item| {
                                    if let MainScreenListItem::File(f) = item {
                                        f.file_name == path
                                    } else {
                                        false
                                    }
                                }) {
                                    state.main_screen.file_cursor = index;
                                }
                            }
                            FocusedPane::Unstaged => {
                                if let Some(index) = state.unstaged_pane.list_items.iter().position(|item| {
                                    if let crate::ui::main_screen::UnstagedListItem::File(f) = item {
                                        f.file_name == path
                                    } else {
                                        false
                                    }
                                }) {
                                    state.unstaged_pane.cursor = index;
                                }
                            }
                        }
                    }
                }
            }
            Input::Character('\u{3}') => {
                let _ = commit_storage::save_commit_message(
                    &state.repo_path,
                    &state.main_screen.commit_message,
                );
                state.running = false;
                return state;
            }
            Input::Character('Q') => {
                if !state.is_in_input_mode() {
                    let _ = commit_storage::save_commit_message(
                        &state.repo_path,
                        &state.main_screen.commit_message,
                    );
                    state.running = false;
                    return state;
                } else {
                    main_screen::handle_input(&mut state, input, max_y, max_x);
                }
            }
            Input::Character('<') => {
                if !state.is_in_input_mode() {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.undo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff();
                    }
                    return state;
                }
            }
            Input::Character('>') => {
                if !state.is_in_input_mode() {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.redo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff();
                    }
                    return state;
                }
            }
            _ => {
                main_screen::handle_input(&mut state, input, max_y, max_x);
            }
        }
    }

    if state.main_screen.has_unstaged_changes == false && state.focused_pane == FocusedPane::Unstaged {
        state.focused_pane = FocusedPane::Main;
    }

    state
}
