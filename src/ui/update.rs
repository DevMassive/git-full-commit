use crate::app_state::{AppState, EditorRequest, Screen};
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, Command, DiscardHunkCommand, IgnoreFileCommand,
    RemoveFileCommand, StageAllCommand, UnstageAllCommand, UnstageFileCommand,
};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::git;
use crate::ui::commit_view;
use crate::ui::diff_view::LINE_CONTENT_OFFSET;
use crate::ui::scroll;
use crate::ui::unstaged_view;
use pancurses::Input;
#[cfg(not(test))]
use pancurses::curs_set;

use crate::git_patch;

fn unstage_line(state: &mut AppState, max_y: i32) {
    if let Some(file) = state.current_file() {
        let line_index = state.line_cursor;
        if let Some(patch) = git_patch::create_unstage_line_patch(file, line_index, true) {
            let command = Box::new(ApplyPatchCommand::new(state.repo_path.clone(), patch));
            let old_line_cursor = state.line_cursor;
            state.execute_and_refresh(command);

            if let Some(file) = state.current_file() {
                state.line_cursor = old_line_cursor.min(file.lines.len().saturating_sub(1));
                let header_height = state.main_header_height(max_y).0;
                let content_height = (max_y as usize).saturating_sub(header_height);
                if state.line_cursor >= state.scroll + content_height {
                    state.scroll = state.line_cursor - content_height + 1;
                }
            }
        }
    }
}

fn handle_commands(state: &mut AppState, input: Input, max_y: i32) -> bool {
    match input {
        Input::Character('q') => {
            if state.is_diff_cursor_active {
                state.is_diff_cursor_active = false;
            } else {
                let _ =
                    commit_storage::save_commit_message(&state.repo_path, &state.commit_message);
                state.running = false;
            }
        }
        Input::Character('i') => {
            if let Some(file) = state.current_file().cloned() {
                if file.file_name != ".gitignore" {
                    let command = Box::new(IgnoreFileCommand::new(
                        state.repo_path.clone(),
                        file.file_name.clone(),
                    ));
                    state.execute_and_refresh(command);
                }
            }
        }
        Input::Character('!') => {
            if state.is_diff_cursor_active {
                if let Some(file) = state.current_file() {
                    let line_index = state.line_cursor;
                    if let Some(hunk) = git_patch::find_hunk(file, line_index) {
                        let patch = git_patch::create_unstage_hunk_patch(file, hunk);
                        let command =
                            Box::new(DiscardHunkCommand::new(state.repo_path.clone(), patch));
                        state.execute_and_refresh(command);
                    }
                }
            } else if let Some(file) = state.current_file().cloned() {
                let patch = git::get_file_diff_patch(&state.repo_path, &file.file_name)
                    .expect("Failed to get diff for file.");
                let command: Box<dyn Command> = if file.status == git::FileStatus::Added {
                    Box::new(RemoveFileCommand::new(
                        state.repo_path.clone(),
                        file.file_name.clone(),
                        patch,
                    ))
                } else {
                    Box::new(CheckoutFileCommand::new(
                        state.repo_path.clone(),
                        file.file_name.clone(),
                        patch,
                    ))
                };
                state.execute_and_refresh(command);
            }
        }
        Input::Character('\n') => {
            if state.file_cursor == 0 {
                let command = Box::new(UnstageAllCommand::new(state.repo_path.clone()));
                state.execute_and_refresh(command);
            } else if let Some(file) = state.current_file().cloned() {
                let line_index = state.line_cursor;
                if let Some(hunk) = git_patch::find_hunk(&file, line_index) {
                    let patch = git_patch::create_unstage_hunk_patch(&file, hunk);
                    let command = Box::new(ApplyPatchCommand::new(state.repo_path.clone(), patch));
                    state.execute_and_refresh(command);
                } else {
                    let command = Box::new(UnstageFileCommand::new(
                        state.repo_path.clone(),
                        file.file_name.clone(),
                    ));
                    state.execute_and_refresh(command);
                }
            }
        }
        Input::Character('1') => unstage_line(state, max_y),
        Input::Character('R') => {
            let command = Box::new(StageAllCommand::new(state.repo_path.clone()));
            state.execute_and_refresh(command);
        }
        Input::Character('e') => {
            if let Some(file) = state.current_file() {
                let line_number = if state.is_diff_cursor_active {
                    git_patch::get_line_number(file, state.line_cursor)
                } else {
                    None
                };
                let file_path = state.repo_path.join(&file.file_name);
                if let Some(path_str) = file_path.to_str() {
                    state.editor_request = Some(EditorRequest {
                        file_path: path_str.to_string(),
                        line_number,
                    });
                }
            }
        }
        _ => return false,
    }
    true
}

pub(crate) fn handle_navigation(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    match input {
        Input::KeyUp | Input::Character('\u{10}') => {
            state.file_cursor = state.file_cursor.saturating_sub(1);
            state.scroll = 0;
            state.line_cursor = 0;
            state.is_diff_cursor_active = false;

            if state.file_cursor < state.file_list_scroll {
                state.file_list_scroll = state.file_cursor;
            }
        }
        Input::KeyDown | Input::Character('\u{e}') => {
            if state.file_cursor < state.files.len() + 2 {
                state.file_cursor += 1;
                state.scroll = 0;
                state.line_cursor = 0;
            }
            state.is_diff_cursor_active = false;

            let file_list_height = state.main_header_height(max_y).0;

            if state.file_cursor >= state.file_list_scroll + file_list_height {
                state.file_list_scroll = state.file_cursor - file_list_height + 1;
            }
        }
        Input::Character('k') => {
            state.is_diff_cursor_active = true;
            state.line_cursor = state.line_cursor.saturating_sub(1);
            let cursor_line = state.get_cursor_line_index();
            if cursor_line < state.scroll {
                state.scroll = cursor_line;
            }
        }
        Input::Character('j') => {
            state.is_diff_cursor_active = true;
            let num_files = state.files.len();
            let lines_count = if state.file_cursor > 0 && state.file_cursor <= num_files {
                state.current_file().map_or(0, |f| f.lines.len())
            } else if state.file_cursor == num_files + 2 {
                state
                    .previous_commit_files
                    .iter()
                    .map(|f| f.lines.len())
                    .sum()
            } else {
                0
            };

            if lines_count > 0 && state.line_cursor < lines_count.saturating_sub(1) {
                state.line_cursor += 1;
                let header_height = state.main_header_height(max_y).0;
                let content_height = (max_y as usize).saturating_sub(header_height);
                let cursor_line = state.get_cursor_line_index();

                if cursor_line >= state.scroll + content_height {
                    state.scroll = cursor_line - content_height + 1;
                }
            }
        }
        Input::KeyLeft => {
            let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);
            state.horizontal_scroll = state.horizontal_scroll.saturating_sub(scroll_amount);
        }
        Input::KeyRight => {
            let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);
            state.horizontal_scroll = state.horizontal_scroll.saturating_add(scroll_amount);
        }
        Input::Character('\t') => {
            if let Some(current_file) = state.current_file() {
                let file_name = current_file.file_name.clone();
                if let Some(index) = state
                    .unstaged_files
                    .iter()
                    .position(|f| f.file_name == file_name)
                {
                    state.unstaged_cursor = index + 1;
                } else if let Some(index) =
                    state.untracked_files.iter().position(|f| *f == file_name)
                {
                    state.unstaged_cursor = state.unstaged_files.len() + index + 2;
                }
            }
            state.screen = Screen::Unstaged;
            state.line_cursor = 0;
            state.unstaged_diff_scroll = 0;
        }
        _ => {
            if state.file_cursor == state.files.len() + 1 {
                commit_view::handle_commit_input(state, input, max_y);
            } else {
                scroll::handle_scroll(state, input, max_y);
            }
        }
    }
}

pub fn update_state(mut state: AppState, input: Option<Input>, max_y: i32, max_x: i32) -> AppState {
    if let Some(input) = input {
        // Global commands
        match input {
            Input::Character('\u{3}') | Input::Character('Q') => {
                let _ =
                    commit_storage::save_commit_message(&state.repo_path, &state.commit_message);
                state.running = false;
                return state;
            }
            Input::Character('<') => {
                if !state.is_commit_mode {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.undo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff();
                    }
                    state.is_commit_mode =
                        state.screen == Screen::Main && state.file_cursor == state.files.len() + 1;
                    return state;
                }
            }
            Input::Character('>') => {
                if !state.is_commit_mode {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.redo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    }
                    state.is_commit_mode =
                        state.screen == Screen::Main && state.file_cursor == state.files.len() + 1;
                    return state;
                }
            }
            _ => (),
        }

        match state.screen {
            Screen::Main => {
                if state.is_commit_mode {
                    commit_view::handle_commit_input(&mut state, input, max_y);
                } else if !handle_commands(&mut state, input, max_y) {
                    handle_navigation(&mut state, input, max_y, max_x);
                }
            }
            Screen::Unstaged => {
                unstaged_view::handle_unstaged_view_input(&mut state, input, max_y);
            }
        }
    }

    state.is_commit_mode =
        state.screen == Screen::Main && state.file_cursor == state.files.len() + 1;

    #[cfg(not(test))]
    if state.is_commit_mode {
        curs_set(1);
    } else {
        curs_set(0);
    }

    state
}
