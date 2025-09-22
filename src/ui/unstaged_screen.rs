use crate::app_state::{AppState, EditorRequest, Screen};
use crate::command::{
    CheckoutFileCommand, DeleteUntrackedFileCommand, DiscardUnstagedHunkCommand,
    IgnoreUnstagedTrackedFileCommand, IgnoreUntrackedFileCommand, StageFileCommand,
    StagePatchCommand, StageUnstagedCommand, StageUntrackedCommand,
};
use crate::git::{self, FileDiff, FileStatus};
use crate::git_patch;
use crate::ui::diff_view::render_diff_view;
use crate::ui::scroll;
use pancurses::{COLOR_PAIR, Input, Window};

pub fn render(window: &Window, state: &AppState) {
    window.erase();
    let (max_y, max_x) = window.get_max_yx();

    let (file_list_height, file_list_total_items) = state.unstaged_header_height(max_y);
    let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
    let untracked_file_count = state.unstaged_screen.untracked_files.len();

    // Render file list
    for i in 0..file_list_height {
        let item_index = state.unstaged_screen.unstaged_scroll + i;
        if item_index >= file_list_total_items {
            break;
        }
        let line_y = i as i32;
        let is_selected = state.unstaged_screen.unstaged_cursor == item_index;
        let pair = if is_selected { 5 } else { 1 };

        if is_selected {
            window.attron(COLOR_PAIR(pair));
            for x in 0..max_x {
                window.mvaddch(line_y, x, ' ');
            }
            window.attroff(COLOR_PAIR(pair));
        }

        window.attron(COLOR_PAIR(pair));
        window.mv(line_y, 0);

        if item_index == 0 {
            window.addstr(" Unstaged changes");
        } else if item_index > 0 && item_index <= unstaged_file_count {
            let file_index = item_index - 1;
            let file = &state.unstaged_screen.unstaged_files[file_index];
            let status_pair = if is_selected { 6 } else { 2 };

            let status_char = match file.status {
                FileStatus::Added => 'A',
                FileStatus::Modified => 'M',
                FileStatus::Renamed => 'R',
                FileStatus::Deleted => 'D',
            };
            window.addstr("   ");
            window.attroff(COLOR_PAIR(pair));
            window.attron(COLOR_PAIR(status_pair));
            window.addstr(format!("{status_char}"));
            window.attroff(COLOR_PAIR(status_pair));
            window.attron(COLOR_PAIR(pair));
            window.addstr(format!(" {}", file.file_name));
        } else if item_index == unstaged_file_count + 1 {
            window.addstr(" Untracked files");
        } else {
            let file_index = item_index - unstaged_file_count - 2;
            let file = &state.unstaged_screen.untracked_files[file_index];
            window.addstr(format!("    ? {file}"));
        }
        window.attroff(COLOR_PAIR(pair));
    }

    // Render separator
    let separator_y = file_list_height as i32;
    window.mv(separator_y, 0);
    window.attron(COLOR_PAIR(9));
    window.hline(pancurses::ACS_HLINE(), max_x);
    window.attroff(COLOR_PAIR(9));

    // Render diff view
    if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
        let file_index = state.unstaged_screen.unstaged_cursor - 1;
        if let Some(file) = state.unstaged_screen.unstaged_files.get(file_index) {
            let content_height = (max_y as usize).saturating_sub(file_list_height + 1);
            render_diff_view(
                window,
                file,
                content_height,
                state.unstaged_screen.unstaged_diff_scroll,
                state.unstaged_screen.unstaged_horizontal_scroll,
                file_list_height + 1,
                state.main_screen.line_cursor,
                state.unstaged_screen.is_unstaged_diff_cursor_active,
            );
        }
    } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1
        && state.unstaged_screen.unstaged_cursor <= unstaged_file_count + 1 + untracked_file_count
    {
        let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
        if let Some(file_path) = state.unstaged_screen.untracked_files.get(file_index) {
            let content_height = (max_y as usize).saturating_sub(file_list_height + 1);

            let lines = match git::read_file_content(&state.repo_path, file_path) {
                Ok((content, size)) => {
                    if is_binary(&content) {
                        vec![format!("  Binary file (size: {} bytes)", size)]
                    } else {
                        String::from_utf8_lossy(&content)
                            .lines()
                            .map(|l| format!(" {l}"))
                            .collect()
                    }
                }
                Err(e) => vec![format!("  Error reading file: {}", e)],
            };

            let file_diff = FileDiff {
                file_name: file_path.clone(),
                status: FileStatus::Added,
                lines,
                hunks: vec![],
            };

            render_diff_view(
                window,
                &file_diff,
                content_height,
                state.unstaged_screen.unstaged_diff_scroll,
                state.unstaged_screen.unstaged_horizontal_scroll,
                file_list_height + 1,
                state.main_screen.line_cursor,
                state.unstaged_screen.is_unstaged_diff_cursor_active,
            );
        }
    }

    #[cfg(not(test))]
    pancurses::curs_set(0);
    window.refresh();
}

fn is_binary(content: &[u8]) -> bool {
    content.contains(&0x00)
}

pub fn handle_input(state: &mut AppState, input: Input, max_y: i32) {
    let (file_list_height, unstaged_items_count) = state.unstaged_header_height(max_y);
    let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
    let _untracked_file_count = state.unstaged_screen.untracked_files.len();

    match input {
        Input::Character('\t') => {
            let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
            let selected_file_name =
                if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
                    state
                        .unstaged_screen.unstaged_files
                        .get(state.unstaged_screen.unstaged_cursor - 1)
                        .map(|f| f.file_name.clone())
                } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1 {
                    let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
                    state.unstaged_screen.untracked_files.get(file_index).cloned()
                } else {
                    None
                };

            if let Some(file_name) = selected_file_name {
                if let Some(index) = state.files.iter().position(|f| f.file_name == file_name) {
                    state.main_screen.file_cursor = index + 1;
                }
            }

            state.screen = Screen::Main;
        }
        Input::Character('q') | Input::Character('Q') => {
            state.screen = Screen::Main;
            state.main_screen.line_cursor = 0;
            state.main_screen.diff_scroll = 0;
        }
        Input::KeyUp => {
            state.unstaged_screen.unstaged_cursor = state.unstaged_screen.unstaged_cursor.saturating_sub(1);
            state.unstaged_screen.unstaged_diff_scroll = 0;
            state.main_screen.line_cursor = 0;
            state.unstaged_screen.is_unstaged_diff_cursor_active = false;
            if state.unstaged_screen.unstaged_cursor < state.unstaged_screen.unstaged_scroll {
                state.unstaged_screen.unstaged_scroll = state.unstaged_screen.unstaged_cursor;
            }
        }
        Input::KeyDown => {
            state.unstaged_screen.unstaged_cursor = state
                .unstaged_screen.unstaged_cursor
                .saturating_add(1)
                .min(unstaged_items_count - 1);
            state.unstaged_screen.unstaged_diff_scroll = 0;
            state.main_screen.line_cursor = 0;
            state.unstaged_screen.is_unstaged_diff_cursor_active = false;
            if state.unstaged_screen.unstaged_cursor >= state.unstaged_screen.unstaged_scroll + file_list_height {
                state.unstaged_screen.unstaged_scroll = state.unstaged_screen.unstaged_cursor - file_list_height + 1;
            }
        }
        Input::Character('k') => {
            state.unstaged_screen.is_unstaged_diff_cursor_active = true;
            state.main_screen.line_cursor = state.main_screen.line_cursor.saturating_sub(1);
            if state.main_screen.line_cursor < state.unstaged_screen.unstaged_diff_scroll {
                state.unstaged_screen.unstaged_diff_scroll = state.main_screen.line_cursor;
            }
        }
        Input::Character('j') => {
            state.unstaged_screen.is_unstaged_diff_cursor_active = true;
            let file_lines_count = if state.unstaged_screen.unstaged_cursor > 0
                && state.unstaged_screen.unstaged_cursor <= unstaged_file_count
            {
                let file_index = state.unstaged_screen.unstaged_cursor - 1;
                state
                    .unstaged_screen.unstaged_files
                    .get(file_index)
                    .map(|f| f.lines.len())
                    .unwrap_or(0)
            } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1 {
                let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
                if let Some(file_path) = state.unstaged_screen.untracked_files.get(file_index) {
                    if let Ok((content, _)) = git::read_file_content(&state.repo_path, file_path) {
                        if is_binary(&content) {
                            1
                        } else {
                            String::from_utf8_lossy(&content).lines().count()
                        }
                    } else {
                        1
                    }
                } else {
                    0
                }
            } else {
                0
            };

            if state.main_screen.line_cursor < file_lines_count.saturating_sub(1) {
                state.main_screen.line_cursor += 1;
                let content_height = (max_y as usize).saturating_sub(file_list_height + 1);
                if state.main_screen.line_cursor >= state.unstaged_screen.unstaged_diff_scroll + content_height {
                    state.unstaged_screen.unstaged_diff_scroll = state.main_screen.line_cursor - content_height + 1;
                }
            }
        }
        Input::KeyLeft => {
            state.unstaged_screen.unstaged_horizontal_scroll = state.unstaged_screen.unstaged_horizontal_scroll.saturating_sub(10);
        }
        Input::KeyRight => {
            state.unstaged_screen.unstaged_horizontal_scroll = state.unstaged_screen.unstaged_horizontal_scroll.saturating_add(10);
        }
        Input::Character('\n') | Input::Character('u') => {
            let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
            if state.unstaged_screen.unstaged_cursor == 0 {
                let command = Box::new(StageUnstagedCommand::new(state.repo_path.clone()));
                state.execute_and_refresh(command);
            } else if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
                let file_index = state.unstaged_screen.unstaged_cursor - 1;
                if let Some(file) = state.unstaged_screen.unstaged_files.get(file_index).cloned() {
                    if state.unstaged_screen.is_unstaged_diff_cursor_active {
                        if let Some(hunk) = git_patch::find_hunk(&file, state.main_screen.line_cursor) {
                            let patch = git_patch::create_stage_hunk_patch(&file, hunk);
                            let command =
                                Box::new(StagePatchCommand::new(state.repo_path.clone(), patch));

                            let old_line_cursor = state.main_screen.line_cursor;
                            state.execute_and_refresh(command);

                            if let Some(updated_file) = state.get_unstaged_file() {
                                state.main_screen.line_cursor =
                                    old_line_cursor.min(updated_file.lines.len().saturating_sub(1));
                                let (file_list_height, _) = state.unstaged_header_height(max_y);
                                let content_height =
                                    (max_y as usize).saturating_sub(file_list_height + 1);
                                if state.main_screen.line_cursor >= state.unstaged_screen.unstaged_diff_scroll + content_height
                                {
                                    state.unstaged_screen.unstaged_diff_scroll =
                                        state.main_screen.line_cursor - content_height + 1;
                                }
                            } else {
                                state.main_screen.line_cursor = 0;
                            }
                        } else {
                            // No hunk found, stage the whole file as a fallback
                            let command = Box::new(StageFileCommand::new(
                                state.repo_path.clone(),
                                file.file_name.clone(),
                            ));
                            state.execute_and_refresh(command);
                        }
                    } else {
                        let command = Box::new(StageFileCommand::new(
                            state.repo_path.clone(),
                            file.file_name.clone(),
                        ));
                        state.execute_and_refresh(command);
                    }
                }
            } else if state.unstaged_screen.unstaged_cursor == unstaged_file_count + 1 {
                let command = Box::new(StageUntrackedCommand::new(state.repo_path.clone()));
                state.execute_and_refresh(command);
            } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1 {
                let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
                if let Some(file_name) = state.unstaged_screen.untracked_files.get(file_index).cloned() {
                    let command =
                        Box::new(StageFileCommand::new(state.repo_path.clone(), file_name));
                    state.execute_and_refresh(command);
                }
            }
        }
        Input::Character('1') => {
            let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
            if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
                let file_index = state.unstaged_screen.unstaged_cursor - 1;
                if let Some(file) = state.unstaged_screen.unstaged_files.get(file_index) {
                    if let Some(patch) = git_patch::create_stage_line_patch(file, state.main_screen.line_cursor)
                    {
                        let command =
                            Box::new(StagePatchCommand::new(state.repo_path.clone(), patch));

                        let old_line_cursor = state.main_screen.line_cursor;
                        state.execute_and_refresh(command);

                        if let Some(updated_file) = state.get_unstaged_file() {
                            state.main_screen.line_cursor =
                                old_line_cursor.min(updated_file.lines.len().saturating_sub(1));
                            let (file_list_height, _) = state.unstaged_header_height(max_y);
                            let content_height =
                                (max_y as usize).saturating_sub(file_list_height + 1);
                            if state.main_screen.line_cursor >= state.unstaged_screen.unstaged_diff_scroll + content_height {
                                state.unstaged_screen.unstaged_diff_scroll = state.main_screen.line_cursor - content_height + 1;
                            }
                        } else {
                            state.main_screen.line_cursor = 0;
                        }
                    }
                }
            }
        }
        Input::Character('e') => {
            let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
            let untracked_file_count = state.unstaged_screen.untracked_files.len();

            if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
                let file_index = state.unstaged_screen.unstaged_cursor - 1;
                if let Some(file) = state.unstaged_screen.unstaged_files.get(file_index) {
                    let line_number = git_patch::get_line_number(file, state.main_screen.line_cursor);
                    let file_path = state.repo_path.join(&file.file_name);
                    if let Some(path_str) = file_path.to_str() {
                        state.editor_request = Some(EditorRequest {
                            file_path: path_str.to_string(),
                            line_number,
                        });
                    }
                }
            } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1
                && state.unstaged_screen.unstaged_cursor <= unstaged_file_count + 1 + untracked_file_count
            {
                let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
                if let Some(file_name) = state.unstaged_screen.untracked_files.get(file_index) {
                    let file_path = state.repo_path.join(file_name);
                    if let Some(path_str) = file_path.to_str() {
                        state.editor_request = Some(EditorRequest {
                            file_path: path_str.to_string(),
                            line_number: None,
                        });
                    }
                }
            }
        }
        Input::Character('!') => {
            let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
            if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
                // Discard unstaged (but tracked) file or hunk
                let file_index = state.unstaged_screen.unstaged_cursor - 1;
                if let Some(file) = state.unstaged_screen.unstaged_files.get(file_index).cloned() {
                    if state.unstaged_screen.is_unstaged_diff_cursor_active {
                        if let Some(hunk) = git_patch::find_hunk(&file, state.main_screen.line_cursor) {
                            let patch = git_patch::create_unstage_hunk_patch(&file, hunk);
                            let command = Box::new(DiscardUnstagedHunkCommand::new(
                                state.repo_path.clone(),
                                patch,
                            ));
                            state.execute_and_refresh(command);
                        }
                    } else {
                        let patch =
                            git::get_unstaged_file_diff_patch(&state.repo_path, &file.file_name)
                                .unwrap_or_default();
                        let command = Box::new(CheckoutFileCommand::new(
                            state.repo_path.clone(),
                            file.file_name.clone(),
                            patch,
                        ));
                        state.execute_and_refresh(command);
                    }
                }
            } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1 {
                // Discard untracked file
                let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
                if let Some(file_name) = state.unstaged_screen.untracked_files.get(file_index).cloned() {
                    if let Ok((content, _)) = git::read_file_content(&state.repo_path, &file_name) {
                        if is_binary(&content) {
                            return; // Do not delete binary files
                        }
                        let command = Box::new(DeleteUntrackedFileCommand::new(
                            state.repo_path.clone(),
                            file_name,
                            content,
                        ));
                        state.execute_and_refresh(command);
                    }
                }
            }
        }
        Input::Character('i') => {
            let unstaged_file_count = state.unstaged_screen.unstaged_files.len();
            let mut file_to_ignore: Option<String> = None;
            let is_tracked;

            if state.unstaged_screen.unstaged_cursor > 0 && state.unstaged_screen.unstaged_cursor <= unstaged_file_count {
                let file_index = state.unstaged_screen.unstaged_cursor - 1;
                if let Some(file) = state.unstaged_screen.unstaged_files.get(file_index) {
                    file_to_ignore = Some(file.file_name.clone());
                }
                is_tracked = true;
            } else if state.unstaged_screen.unstaged_cursor > unstaged_file_count + 1 {
                let file_index = state.unstaged_screen.unstaged_cursor - unstaged_file_count - 2;
                if let Some(file_name) = state.unstaged_screen.untracked_files.get(file_index) {
                    file_to_ignore = Some(file_name.clone());
                }
                is_tracked = false;
            } else {
                return; // Header selected, do nothing
            }

            if let Some(file_name) = file_to_ignore {
                if file_name != ".gitignore" {
                    let command: Box<dyn crate::command::Command> = if is_tracked {
                        Box::new(IgnoreUnstagedTrackedFileCommand::new(
                            state.repo_path.clone(),
                            file_name,
                        ))
                    } else {
                        Box::new(IgnoreUntrackedFileCommand::new(
                            state.repo_path.clone(),
                            file_name,
                        ))
                    };
                    state.execute_and_refresh(command);
                }
            }
        }
        _ => scroll::handle_scroll(state, input, max_y),
    }
}

// All tests are now in the main test file
