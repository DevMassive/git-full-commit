use crate::app_state::{AppState, EditorRequest, Screen};
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, Command, DiscardHunkCommand, IgnoreFileCommand,
    RemoveFileCommand, StageAllCommand, UnstageAllCommand, UnstageFileCommand,
};
use crate::commit_storage;
use crate::git;
use crate::ui::commit_view;
use crate::ui::diff_view;
use crate::ui::diff_view::LINE_CONTENT_OFFSET;
use crate::ui::scroll;
use pancurses::{A_DIM, Input};

use crate::git::FileStatus;
use pancurses::{COLOR_PAIR, Window};

use crate::git_patch;

pub fn render(window: &Window, state: &AppState) {
    let (max_y, max_x) = window.get_max_yx();

    let (file_list_height, file_list_total_items) = state.main_header_height(max_y);
    let num_files = state.files.len();

    let mut carret_y = 0;
    let mut carret_x = 0;

    for i in 0..file_list_height {
        let item_index = state.main_screen.file_list_scroll + i;
        if item_index >= file_list_total_items {
            break;
        }
        let line_y = i as i32;
        let is_selected = state.main_screen.file_cursor == item_index;

        if item_index == 0 {
            // Render "Staged changes"
            let pair = if is_selected { 5 } else { 1 };
            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);
            if state.main_screen.has_unstaged_changes {
                window.attron(A_DIM);
                window.addstr(&" Unstaged changes |".to_string());
                window.attroff(A_DIM);
            }
            window.addstr(&" Staged changes".to_string());
            window.attroff(COLOR_PAIR(pair));
        } else if item_index > 0 && item_index <= num_files {
            let file_index = item_index - 1;
            let file = &state.files[file_index];
            let pair = if is_selected { 5 } else { 1 };
            let status_pair = if is_selected { 6 } else { 2 };

            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);
            window.attroff(COLOR_PAIR(pair));

            let status_char = match file.status {
                FileStatus::Added => 'A',
                FileStatus::Modified => 'M',
                FileStatus::Renamed => 'R',
                FileStatus::Deleted => 'D',
            };
            window.attron(COLOR_PAIR(pair));
            window.addstr("   ");
            window.attroff(COLOR_PAIR(pair));
            window.attron(COLOR_PAIR(status_pair));
            window.addstr(format!("{status_char}"));
            window.attroff(COLOR_PAIR(status_pair));
            window.attron(COLOR_PAIR(pair));
            if file.status == FileStatus::Renamed {
                window.addstr(format!(" {} -> {}", file.old_file_name, file.file_name));
            } else {
                window.addstr(format!(" {}", file.file_name));
            }
            window.attroff(COLOR_PAIR(pair));
        } else if item_index == num_files + 1 {
            // Render commit message line
            (carret_x, carret_y) = commit_view::render(window, state, is_selected, line_y, max_x);
        } else if item_index == num_files + 2 {
            // Render previous commit info
            let pair = if is_selected { 5 } else { 1 };
            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);
            if state.main_screen.is_amend_mode {
                window.addstr(" |");
            } else {
                let status = if state.previous_commit_is_on_remote {
                    "(remote)"
                } else {
                    "(local)"
                };
                window.addstr(format!(" o {} {}", status, &state.previous_commit_message));
            }
            window.attroff(COLOR_PAIR(pair));
        }
    }

    // Render separator
    let separator_y = file_list_height as i32;
    window.mv(separator_y, 0);
    window.attron(COLOR_PAIR(9));
    window.hline(pancurses::ACS_HLINE(), max_x);
    window.attroff(COLOR_PAIR(9));

    let header_height = file_list_height + 1;
    let content_height = (max_y as usize).saturating_sub(header_height);
    let cursor_position = state.get_cursor_line_index();

    if state.main_screen.file_cursor == 0 {
        // "Staged changes" is selected, do nothing for now.
    } else if state.main_screen.file_cursor == num_files + 2 {
        // Render previous commit diff
        let content_height = (max_y as usize).saturating_sub(file_list_height + 1);

        diff_view::render_multiple(
            window,
            &state.previous_commit_files,
            content_height,
            state.main_screen.diff_scroll,
            state.main_screen.horizontal_scroll,
            header_height,
            cursor_position,
            state.main_screen.is_diff_cursor_active,
        );
    } else if state.main_screen.file_cursor > 0 && state.main_screen.file_cursor <= num_files {
        let selected_file = &state.files[state.main_screen.file_cursor - 1];
        diff_view::render(
            window,
            selected_file,
            content_height,
            state.main_screen.diff_scroll,
            state.main_screen.horizontal_scroll,
            header_height,
            cursor_position,
            state.main_screen.is_diff_cursor_active,
        );
    }

    window.mv(carret_y, carret_x);
    if state.main_screen.file_cursor == num_files + 1 {
        #[cfg(not(test))]
        pancurses::curs_set(1);
    } else {
        #[cfg(not(test))]
        pancurses::curs_set(0);
    }
}

pub fn handle_input(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    if state.main_screen.is_commit_mode {
        commit_view::handle_commit_input(state, input, max_y);
    } else if !handle_commands(state, input, max_y) {
        handle_navigation(state, input, max_y, max_x);
    }
}

fn unstage_line(state: &mut AppState, max_y: i32) {
    if let Some(file) = state.current_file() {
        let line_index = state.main_screen.line_cursor;
        if let Some(patch) = git_patch::create_unstage_line_patch(file, line_index, true) {
            let command = Box::new(ApplyPatchCommand::new(state.repo_path.clone(), patch));
            let old_line_cursor = state.main_screen.line_cursor;
            state.execute_and_refresh(command);

            if let Some(file) = state.current_file() {
                state.main_screen.line_cursor =
                    old_line_cursor.min(file.lines.len().saturating_sub(1));
                let header_height = state.main_header_height(max_y).0;
                let content_height = (max_y as usize).saturating_sub(header_height);
                if state.main_screen.line_cursor >= state.main_screen.diff_scroll + content_height {
                    state.main_screen.diff_scroll =
                        state.main_screen.line_cursor - content_height + 1;
                }
            }
        }
    }
}

fn handle_commands(state: &mut AppState, input: Input, max_y: i32) -> bool {
    match input {
        Input::Character('q') => {
            if state.main_screen.is_diff_cursor_active {
                state.main_screen.is_diff_cursor_active = false;
            } else {
                let _ = commit_storage::save_commit_message(
                    &state.repo_path,
                    &state.main_screen.commit_message,
                );
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
            if state.main_screen.is_diff_cursor_active {
                if let Some(file) = state.current_file() {
                    let line_index = state.main_screen.line_cursor;
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
        Input::Character('\n') | Input::Character('u') => {
            if state.main_screen.file_cursor == 0 {
                let command = Box::new(UnstageAllCommand::new(state.repo_path.clone()));
                state.execute_and_refresh(command);
            } else if let Some(file) = state.current_file().cloned() {
                let line_index = state.main_screen.line_cursor;
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
                let line_number = if state.main_screen.is_diff_cursor_active {
                    git_patch::get_line_number(file, state.main_screen.line_cursor)
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

fn handle_navigation(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    match input {
        Input::KeyUp | Input::Character('\u{10}') => {
            state.main_screen.file_cursor = state.main_screen.file_cursor.saturating_sub(1);
            state.main_screen.diff_scroll = 0;
            state.main_screen.line_cursor = 0;
            state.main_screen.is_diff_cursor_active = false;

            if state.main_screen.file_cursor < state.main_screen.file_list_scroll {
                state.main_screen.file_list_scroll = state.main_screen.file_cursor;
            }
        }
        Input::KeyDown | Input::Character('\u{e}') => {
            if state.main_screen.file_cursor < state.files.len() + 2 {
                state.main_screen.file_cursor += 1;
                state.main_screen.diff_scroll = 0;
                state.main_screen.line_cursor = 0;
            }
            state.main_screen.is_diff_cursor_active = false;

            let file_list_height = state.main_header_height(max_y).0;

            if state.main_screen.file_cursor
                >= state.main_screen.file_list_scroll + file_list_height
            {
                state.main_screen.file_list_scroll =
                    state.main_screen.file_cursor - file_list_height + 1;
            }
        }
        Input::Character('k') => {
            state.main_screen.is_diff_cursor_active = true;
            state.main_screen.line_cursor = state.main_screen.line_cursor.saturating_sub(1);
            let cursor_line = state.get_cursor_line_index();
            if cursor_line < state.main_screen.diff_scroll {
                state.main_screen.diff_scroll = cursor_line;
            }
        }
        Input::Character('j') => {
            state.main_screen.is_diff_cursor_active = true;
            let num_files = state.files.len();
            let lines_count = if state.main_screen.file_cursor > 0
                && state.main_screen.file_cursor <= num_files
            {
                state.current_file().map_or(0, |f| f.lines.len())
            } else if state.main_screen.file_cursor == num_files + 2 {
                state
                    .previous_commit_files
                    .iter()
                    .map(|f| f.lines.len())
                    .sum()
            } else {
                0
            };

            if lines_count > 0 && state.main_screen.line_cursor < lines_count.saturating_sub(1) {
                state.main_screen.line_cursor += 1;
                let header_height = state.main_header_height(max_y).0;
                let content_height = (max_y as usize).saturating_sub(header_height);
                let cursor_line = state.get_cursor_line_index();

                if cursor_line >= state.main_screen.diff_scroll + content_height {
                    state.main_screen.diff_scroll = cursor_line - content_height + 1;
                }
            }
        }
        Input::KeyLeft => {
            let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);
            state.main_screen.horizontal_scroll = state
                .main_screen
                .horizontal_scroll
                .saturating_sub(scroll_amount);
        }
        Input::KeyRight => {
            let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);
            state.main_screen.horizontal_scroll = state
                .main_screen
                .horizontal_scroll
                .saturating_add(scroll_amount);
        }
        Input::Character('\t') => {
            if !state.main_screen.has_unstaged_changes {
                return;
            }
            if let Some(current_file) = state.current_file() {
                let file_name = current_file.file_name.clone();
                if let Some(index) = state
                    .unstaged_screen
                    .unstaged_files
                    .iter()
                    .position(|f| f.file_name == file_name)
                {
                    state.unstaged_screen.unstaged_cursor = index + 1;
                } else if let Some(index) = state
                    .unstaged_screen
                    .untracked_files
                    .iter()
                    .position(|f| *f == file_name)
                {
                    state.unstaged_screen.unstaged_cursor =
                        state.unstaged_screen.unstaged_files.len() + index + 2;
                }
            }
            state.screen = Screen::Unstaged;
        }
        _ => {
            if state.main_screen.file_cursor == state.files.len() + 1 {
                commit_view::handle_commit_input(state, input, max_y);
            } else {
                scroll::handle_scroll(state, input, max_y);
            }
        }
    }
}
