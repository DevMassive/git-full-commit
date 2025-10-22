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
                        FocusedPane::Unstaged => {
                            state.get_unstaged_file().map(|f| f.file_name.clone())
                        }
                    };

                    state.focused_pane = match state.focused_pane {
                        FocusedPane::Main => FocusedPane::Unstaged,
                        FocusedPane::Unstaged => FocusedPane::Main,
                    };

                    if let Some(path) = current_file_path {
                        match state.focused_pane {
                            FocusedPane::Main => {
                                if let Some(index) =
                                    state.main_screen.list_items.iter().position(|item| {
                                        if let MainScreenListItem::File(f) = item {
                                            f.file_name == path
                                        } else {
                                            false
                                        }
                                    })
                                {
                                    state.main_screen.file_cursor = index;
                                }
                            }
                            FocusedPane::Unstaged => {
                                if let Some(index) =
                                    state.unstaged_pane.list_items.iter().position(|item| {
                                        if let crate::ui::main_screen::UnstagedListItem::File(f) =
                                            item
                                        {
                                            f.file_name == path
                                        } else {
                                            false
                                        }
                                    })
                                {
                                    state.unstaged_pane.cursor = index;
                                }
                            }
                        }
                    }
                }
            }
            Input::Character('\u{3}') => {
                // Ctrl+C
                if state.main_screen.is_reordering_commits {
                    // In reorder mode, Ctrl+C is used to cancel edits, so pass it down.
                    main_screen::handle_input(&mut state, input, max_y, max_x);
                } else {
                    // Otherwise, it's a global quit command.
                    let _ = commit_storage::save_commit_message(
                        &state.repo_path,
                        &state.main_screen.commit_message,
                    );
                    state.running = false;
                    return state;
                }
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
                        state.refresh_diff(false);
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff(false);
                    }
                    return state;
                }
            }
            Input::Character('>') => {
                if !state.is_in_input_mode() {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.redo(cursor_state) {
                        state.refresh_diff(false);
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff(false);
                    }
                    return state;
                }
            }
            _ => {
                main_screen::handle_input(&mut state, input, max_y, max_x);
            }
        }
    }

    if !state.main_screen.has_unstaged_changes && state.focused_pane == FocusedPane::Unstaged {
        state.focused_pane = FocusedPane::Main;
    }

    state
}

use crate::command::SwapCommitCommand;

fn is_item_on_remote(item: &MainScreenListItem) -> bool {
    match item {
        MainScreenListItem::PreviousCommitInfo { is_on_remote, .. } => *is_on_remote,
        MainScreenListItem::EditingReorderCommit { is_on_remote, .. } => *is_on_remote,
        _ => false,
    }
}

pub fn update_state_with_alt(
    mut state: AppState,
    input: Option<Input>,
    max_y: i32,
    max_x: i32,
) -> AppState {
    if let Some(input) = input {
        if state.is_in_input_mode() {
            main_screen::handle_alt_input(&mut state, input, max_y, max_x);
            return state;
        }

        if !state.main_screen.is_reordering_commits {
            if let Some(MainScreenListItem::PreviousCommitInfo { .. }) = state.current_main_item() {
                match input {
                    Input::KeyUp | Input::KeyDown => {
                        main_screen::start_reorder_mode(&mut state);
                    }
                    Input::Character('\n') => {
                        if let Some(MainScreenListItem::PreviousCommitInfo {
                            hash,
                            message,
                            is_on_remote,
                            is_fixup,
                        }) = state.current_main_item().cloned()
                        {
                            if !is_on_remote {
                                main_screen::start_reorder_mode(&mut state);
                                let current_index = state.main_screen.file_cursor;
                                if let Some(item) =
                                    state.main_screen.list_items.get_mut(current_index)
                                {
                                    *item = MainScreenListItem::EditingReorderCommit {
                                        hash,
                                        original_message: message.clone(),
                                        current_text: message.clone(),
                                        cursor: message.chars().count(),
                                        is_on_remote,
                                        is_fixup,
                                    };
                                }
                            }
                        }
                        return state;
                    }
                    _ => {}
                }
            }
        }

        if state.main_screen.is_reordering_commits {
            match input {
                Input::KeyUp => {
                    let cursor = state.main_screen.file_cursor;
                    if cursor > 0 {
                        let can_swap = match (
                            state.main_screen.list_items.get(cursor),
                            state.main_screen.list_items.get(cursor - 1),
                        ) {
                            (Some(item1), Some(item2)) => {
                                !is_item_on_remote(item1) && !is_item_on_remote(item2)
                            }
                            _ => false,
                        };

                        if can_swap {
                            let command = Box::new(SwapCommitCommand::new(
                                &mut state.main_screen.list_items,
                                cursor,
                                cursor - 1,
                            ));
                            state.execute_reorder_command(command);
                            state.main_screen.file_cursor -= 1;
                        }
                    }
                }
                Input::KeyDown => {
                    let cursor = state.main_screen.file_cursor;
                    if cursor < state.main_screen.list_items.len() - 1 {
                        let can_swap = match (
                            state.main_screen.list_items.get(cursor),
                            state.main_screen.list_items.get(cursor + 1),
                        ) {
                            (Some(item1), Some(item2)) => {
                                !is_item_on_remote(item1) && !is_item_on_remote(item2)
                            }
                            _ => false,
                        };

                        if can_swap {
                            let command = Box::new(SwapCommitCommand::new(
                                &mut state.main_screen.list_items,
                                cursor,
                                cursor + 1,
                            ));
                            state.execute_reorder_command(command);
                            state.main_screen.file_cursor += 1;
                        }
                    }
                }
                Input::Character('\n') => {
                    let current_index = state.main_screen.file_cursor;
                    if let Some(MainScreenListItem::PreviousCommitInfo {
                        hash,
                        message,
                        is_on_remote,
                        is_fixup,
                    }) = state.main_screen.list_items.get(current_index).cloned()
                    {
                        if !is_on_remote {
                            if let Some(item) = state.main_screen.list_items.get_mut(current_index)
                            {
                                *item = MainScreenListItem::EditingReorderCommit {
                                    hash,
                                    original_message: message.clone(),
                                    current_text: message.clone(),
                                    cursor: message.chars().count(),
                                    is_on_remote,
                                    is_fixup,
                                };
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    state
}
