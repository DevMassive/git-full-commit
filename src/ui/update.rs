use crate::app_state::{AppState, Screen};
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, Command, DiscardHunkCommand, IgnoreFileCommand,
    RemoveFileCommand, StageAllCommand, UnstageAllCommand, UnstageFileCommand,
};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::external_command;
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
                    let _ = external_command::open_editor(path_str, line_number);
                }
            }
        }
        _ => return false,
    }
    true
}

fn handle_navigation(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    match input {
        Input::KeyUp => {
            state.file_cursor = state.file_cursor.saturating_sub(1);
            state.scroll = 0;
            state.line_cursor = 0;
            state.is_diff_cursor_active = false;

            if state.file_cursor < state.file_list_scroll {
                state.file_list_scroll = state.file_cursor;
            }
        }
        Input::KeyDown => {
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
                } else {
                    state.unstaged_cursor = 1;
                }
            } else {
                state.unstaged_cursor = 1;
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
            Input::Character('u') => {
                if !state.is_commit_mode {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.undo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    } else {
                        state.refresh_diff();
                    }
                    state.is_commit_mode = state.screen == Screen::Main
                        && state.file_cursor == state.files.len() + 1;
                    return state;
                }
            }
            Input::Character('r') => {
                if !state.is_commit_mode {
                    let cursor_state = CursorState::from_app_state(&state);
                    if let Some(cursor) = state.command_history.redo(cursor_state) {
                        state.refresh_diff();
                        cursor.apply_to_app_state(&mut state);
                    }
                    state.is_commit_mode = state.screen == Screen::Main
                        && state.file_cursor == state.files.len() + 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::external_command;
    use crate::git::{FileDiff, FileStatus, Hunk};
    use pancurses::Input;
    use std::path::PathBuf;
    use std::process::Command as OsCommand;

    // Helper function to create a temporary git repository for testing
    fn setup_temp_repo() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("test_repo_{}", rand::random::<u32>()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();

        OsCommand::new("git")
            .arg("init")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to init git repo");
        OsCommand::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git user.name");
        OsCommand::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git user.email");

        temp_dir
    }

    fn get_git_status(repo_path: &PathBuf) -> String {
        let output = OsCommand::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(repo_path)
            .output()
            .expect("Failed to get git status");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn create_test_file_diff() -> FileDiff {
        let lines = vec![
            "@@ -1,5 +1,6 @@".to_string(),
            " line 1".to_string(),
            "-line 2".to_string(),
            "-line 3".to_string(),
            "+line 2 new".to_string(),
            "+line 3 new".to_string(),
            " line 4".to_string(),
        ];
        let line_numbers = vec![
            (0, 0), // @@
            (1, 1), // ` line 1`
            (2, 1), // `-line 2`
            (3, 1), // `-line 3`
            (3, 2), // `+line 2 new`
            (3, 3), // `+line 3 new`
            (4, 4), // ` line 4`
        ];
        let hunks = vec![Hunk {
            start_line: 0,
            lines: lines.clone(),
            old_start: 1,
            new_start: 1,
            line_numbers,
        }];
        FileDiff {
            file_name: "test.txt".to_string(),
            hunks,
            lines,
            status: FileStatus::Modified,
        }
    }

    fn create_state_with_files(num_files: usize) -> AppState {
        let files: Vec<FileDiff> = (0..num_files)
            .map(|i| FileDiff {
                file_name: format!("file_{i}.txt"),
                status: FileStatus::Modified,
                lines: vec![],
                hunks: vec![],
            })
            .collect();

        let mut state = AppState::new(PathBuf::from("/tmp"), files);
        state.previous_commit_files = vec![];
        state
    }

    #[test]
    fn test_file_list_scrolling() {
        let mut state = create_state_with_files(50);
        let max_y = 30; // file_list_height = 10

        // --- Scroll down ---
        // Move to cursor 9, scroll should be 0
        for _ in 0..8 {
            state = update_state(state, Some(Input::KeyDown), max_y, 80);
        }
        assert_eq!(state.file_cursor, 9);
        assert_eq!(state.file_list_scroll, 0);

        // Move to cursor 10, scroll should be 1
        state = update_state(state, Some(Input::KeyDown), max_y, 80);
        assert_eq!(state.file_cursor, 10);
        assert_eq!(state.file_list_scroll, 1);

        // Move to cursor 20, scroll should be 11
        for _ in 0..10 {
            state = update_state(state, Some(Input::KeyDown), max_y, 80);
        }
        assert_eq!(state.file_cursor, 20);
        assert_eq!(state.file_list_scroll, 11);

        // --- Scroll up ---
        // Move to cursor 11, scroll should be 11
        for _ in 0..9 {
            state = update_state(state, Some(Input::KeyUp), max_y, 80);
        }
        assert_eq!(state.file_cursor, 11);
        assert_eq!(state.file_list_scroll, 11);

        // Move to cursor 10, scroll should be 10
        state = update_state(state, Some(Input::KeyUp), max_y, 80);
        assert_eq!(state.file_cursor, 10);
        assert_eq!(state.file_list_scroll, 10);

        // Move to cursor 0, scroll should be 0
        for _ in 0..10 {
            state = update_state(state, Some(Input::KeyUp), max_y, 80);
        }
        assert_eq!(state.file_cursor, 0);
        assert_eq!(state.file_list_scroll, 0);
    }

    fn create_test_state(
        lines_count: usize,
        file_cursor: usize,
        line_cursor: usize,
        scroll: usize,
    ) -> AppState {
        let mut files = Vec::new();
        if lines_count > 0 {
            let lines = (0..lines_count).map(|i| format!("line {i}")).collect();
            files.push(FileDiff {
                file_name: "test_file.rs".to_string(),
                status: FileStatus::Modified,
                lines,
                hunks: vec![Hunk {
                    old_start: 1,
                    new_start: 1,
                    lines: Vec::new(),
                    start_line: 0,
                    line_numbers: Vec::new(),
                }],
            });
        }

        let mut state = AppState::new(PathBuf::from("/tmp"), files);
        state.file_cursor = file_cursor;
        state.line_cursor = line_cursor;
        state.scroll = scroll;
        // Mock previous commit files to avoid git command execution in tests
        state.previous_commit_files = vec![];
        state
    }

    // --- Page Down Tests ---

    #[test]
    fn test_page_down_scrolls_by_page() {
        let initial_state = create_test_state(100, 1, 5, 0);
        let max_y = 30;
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height); // 25

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y, 80);

        let expected_cursor = 5 + content_height;
        assert_eq!(
            final_state.line_cursor, expected_cursor,
            "Cursor should move down by one page"
        );
        // Scroll should jump by a page, not just follow the cursor
        assert_eq!(
            final_state.scroll, content_height,
            "Scroll should move by a full page"
        );
    }

    #[test]
    fn test_page_down_scrolls_beyond_content() {
        let lines_count: usize = 100;
        let max_y = 30;
        let initial_state = create_test_state(lines_count, 1, 95, 74);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height); // 25

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y, 80);

        assert_eq!(
            final_state.line_cursor, 99,
            "Cursor should move to the last line"
        );
        assert_eq!(
            final_state.scroll,
            74 + content_height,
            "Scroll should increase by a page even if it goes beyond max_scroll"
        );
    }

    #[test]
    fn test_page_down_clamps_at_end() {
        let lines_count: usize = 40;
        let max_y = 30;
        let initial_state = create_test_state(lines_count, 1, 10, 0);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height); // 25

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y, 80);

        let expected_cursor = (10 + content_height).min(lines_count - 1); // 35
        assert_eq!(final_state.line_cursor, expected_cursor);

        assert_eq!(final_state.scroll, content_height); // 25
    }

    // --- Page Up Tests ---

    #[test]
    fn test_page_up_scrolls_by_page() {
        let max_y = 30;
        let initial_state = create_test_state(100, 1, 60, 50);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height); // 25

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

        let expected_cursor = 60 - content_height;
        assert_eq!(
            final_state.line_cursor, expected_cursor,
            "Cursor should move up by one page"
        );
        assert_eq!(
            final_state.scroll,
            50 - content_height, // 25
            "Scroll should move up by a full page"
        );
    }

    #[test]
    fn test_page_up_stops_at_top() {
        let max_y = 30;
        let initial_state = create_test_state(100, 1, 20, 15);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height); // 25

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

        assert_eq!(
            final_state.line_cursor,
            20_usize.saturating_sub(content_height), // 0
            "Cursor should move up by one page or saturate at 0"
        );
        assert_eq!(final_state.scroll, 0, "Scroll should clamp at the top");
    }

    #[test]
    fn test_page_up_at_top_does_nothing() {
        let max_y = 30;
        let _content_height = (max_y as usize).saturating_sub(1 + 4);
        let initial_state = create_test_state(100, 1, 10, 0);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

        assert_eq!(final_state.scroll, 0, "Scroll should not change");
        assert_eq!(
            final_state.line_cursor, 0,
            "Cursor should be at the first line"
        );
    }

    #[test]
    fn test_ignore_file() {
        // Setup a temporary git repository
        let temp_dir = std::env::temp_dir().join("test_repo_for_ignore_v2");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();
        OsCommand::new("git")
            .arg("init")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to init git repo");
        OsCommand::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git user.name");
        OsCommand::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git user.email");
        std::fs::write(temp_dir.join("a.txt"), "initial content").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg("a.txt")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to git add");
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to git commit");

        // Create a file to be ignored
        let file_to_ignore = "some_file.txt";
        std::fs::write(temp_dir.join(file_to_ignore), "Hello").unwrap();

        // Stage the file
        OsCommand::new("git")
            .arg("add")
            .arg(file_to_ignore)
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to git add");

        // Create initial state
        let files = crate::git::get_diff(temp_dir.clone());
        let mut state = AppState::new(temp_dir.clone(), files);
        state.file_cursor = 1; // Select the file

        // Simulate pressing 'i'
        let mut updated_state = update_state(state, Some(Input::Character('i')), 80, 80);

        // Check if .gitignore is correct
        let gitignore_path = temp_dir.join(".gitignore");
        assert!(gitignore_path.exists(), ".gitignore should be created");
        let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
        assert!(
            gitignore_content.contains(file_to_ignore),
            ".gitignore should contain the ignored file"
        );

        // After ignoring, the file should be gone from the diff,
        // and the .gitignore file should be the only change.
        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should only contain .gitignore"
        );
        assert_eq!(
            updated_state.files[0].file_name, ".gitignore",
            "The remaining file should be .gitignore"
        );

        // Simulate undo
        updated_state = update_state(updated_state, Some(Input::Character('u')), 80, 80);

        // After undo, the original file should be back and .gitignore should be gone
        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should contain the original file again"
        );
        assert_eq!(
            updated_state.files[0].file_name, file_to_ignore,
            "The file should be the one we ignored"
        );

        // Simulate undo again
        let updated_state = update_state(updated_state, Some(Input::Character('u')), 80, 80);

        // After undo, the original file should be back and .gitignore should be gone
        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should contain the original file again"
        );
        assert_eq!(
            updated_state.files[0].file_name, file_to_ignore,
            "The file should be the one we ignored"
        );

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_half_page_down() {
        let lines_count: usize = 100;
        let max_y = 30; // content_height = 25
        let initial_state = create_test_state(lines_count, 1, 10, 5);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height);
        let scroll_amount = (content_height / 2).max(1); // 12

        let final_state = update_state(initial_state, Some(Input::Character('\u{4}')), max_y, 80);

        let expected_cursor = 10 + scroll_amount;
        assert_eq!(final_state.line_cursor, expected_cursor);
        // Cursor is at 22, scroll is 5, content_height is 25. 22 < 5 + 25. No scroll.
        assert_eq!(final_state.scroll, 5);
    }

    #[test]
    fn test_half_page_down_and_scroll() {
        let lines_count: usize = 100;
        let max_y = 30; // content_height = 25
        let initial_state = create_test_state(lines_count, 1, 20, 0);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height);
        let scroll_amount = (content_height / 2).max(1); // 12

        let final_state = update_state(initial_state, Some(Input::Character('\u{4}')), max_y, 80);

        let expected_cursor = 20 + scroll_amount; // 32
        assert_eq!(final_state.line_cursor, expected_cursor);
        // Cursor is at 32, scroll is 0, content_height is 25. 32 >= 0 + 25. Scroll.
        let expected_scroll = scroll_amount;
        assert_eq!(final_state.scroll, expected_scroll);
    }

    #[test]
    fn test_half_page_up() {
        let lines_count: usize = 100;
        let max_y = 30; // content_height = 25
        let initial_state = create_test_state(lines_count, 1, 20, 15);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height);
        let scroll_amount = (content_height / 2).max(1); // 12

        let final_state = update_state(initial_state, Some(Input::Character('\u{15}')), max_y, 80);

        let expected_cursor = 20 - scroll_amount; // 8
        assert_eq!(final_state.line_cursor, expected_cursor);
        // Cursor is at 8, scroll is 15. 8 < 15. Scroll.
        let expected_scroll = 15 - scroll_amount; // 3
        assert_eq!(final_state.scroll, expected_scroll.max(0));
    }

    #[test]
    fn test_half_page_up_and_scroll() {
        let lines_count: usize = 100;
        let max_y = 30; // content_height = 25
        let initial_state = create_test_state(lines_count, 1, 10, 10);
        let (header_height, _) = initial_state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height);
        let scroll_amount = (content_height / 2).max(1); // 12

        let final_state = update_state(initial_state, Some(Input::Character('\u{15}')), max_y, 80);

        let expected_cursor = 10_usize.saturating_sub(scroll_amount); // 0
        assert_eq!(final_state.line_cursor, expected_cursor);
        // Cursor is at 0, scroll is 10. 0 < 10. Scroll.
        let expected_scroll = 10_usize.saturating_sub(scroll_amount); // 0
        assert_eq!(final_state.scroll, expected_scroll);
    }

    #[test]
    fn test_horizontal_scroll() {
        let mut state = create_test_state(10, 1, 0, 0);
        assert_eq!(state.horizontal_scroll, 0);
        let max_x = 80;
        let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);

        // Scroll right
        state = update_state(state, Some(Input::KeyRight), 30, max_x);
        assert_eq!(state.horizontal_scroll, scroll_amount);
        state = update_state(state, Some(Input::KeyRight), 30, max_x);
        assert_eq!(state.horizontal_scroll, scroll_amount * 2);

        // Scroll left
        state = update_state(state, Some(Input::KeyLeft), 30, max_x);
        assert_eq!(state.horizontal_scroll, scroll_amount);
        state = update_state(state, Some(Input::KeyLeft), 30, max_x);
        assert_eq!(state.horizontal_scroll, 0);

        // Scroll left at 0 should not change
        state = update_state(state, Some(Input::KeyLeft), 30, max_x);
        assert_eq!(state.horizontal_scroll, 0);
    }

    #[test]
    fn test_q_behavior_with_active_diff_cursor() {
        let mut state = create_test_state(10, 1, 0, 0);
        state.is_diff_cursor_active = true;

        // First 'q' should only deactivate the cursor
        let state_after_first_q = update_state(state, Some(Input::Character('q')), 30, 80);
        assert!(
            state_after_first_q.running,
            "App should still be running after first 'q'"
        );
        assert!(
            !state_after_first_q.is_diff_cursor_active,
            "Diff cursor should be inactive after first 'q'"
        );

        // Second 'q' should quit the app
        let state_after_second_q =
            update_state(state_after_first_q, Some(Input::Character('q')), 30, 80);
        assert!(
            !state_after_second_q.running,
            "App should quit after second 'q'"
        );
    }

    #[test]
    fn test_discard_hunk() {
        // Setup a temporary git repository
        let temp_dir = std::env::temp_dir().join("test_repo_for_discard_hunk");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();
        let repo_path = temp_dir;

        OsCommand::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to init git repo");
        OsCommand::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to set git user.name");
        OsCommand::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to set git user.email");

        // Create and commit a file
        let file_path = repo_path.join("test.txt");
        let initial_content = (1..=20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&file_path, &initial_content).unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to git add");
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to git commit");

        // Modify the file to create two hunks
        let modified_content = {
            let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
            lines[2] = "modified line 3".to_string(); // Hunk 1
            lines[15] = "modified line 16".to_string(); // Hunk 2
            lines.join("\n")
        };
        std::fs::write(&file_path, &modified_content).unwrap();

        // Stage the changes
        OsCommand::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to git add");

        // Create app state
        let files = crate::git::get_diff(repo_path.clone());
        let mut state = AppState::new(repo_path.clone(), files);
        state.file_cursor = 1; // Select the file
        state.is_diff_cursor_active = true;
        // Move cursor to the second hunk (around line 16)
        // The diff output will have headers and context lines, so we need to estimate the line number
        let line_in_diff = state.files[0]
            .lines
            .iter()
            .position(|l| l.contains("modified line 16"))
            .unwrap_or(15);
        state.line_cursor = line_in_diff;

        // Simulate pressing '!' to discard the hunk
        let updated_state = update_state(state, Some(Input::Character('!')), 80, 80);

        // Check that the hunk is gone from both staged and working directory
        let staged_diff = OsCommand::new("git")
            .arg("diff")
            .arg("--staged")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get staged diff");
        let staged_diff_str = String::from_utf8_lossy(&staged_diff.stdout);
        assert!(!staged_diff_str.contains("modified line 16"));
        assert!(staged_diff_str.contains("modified line 3")); // First hunk should remain

        let working_diff = OsCommand::new("git")
            .arg("diff")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get working directory diff");
        let working_diff_str = String::from_utf8_lossy(&working_diff.stdout);
        assert!(
            working_diff_str.is_empty(),
            "Working directory should be clean"
        );

        // Simulate undo
        let _updated_state = update_state(updated_state, Some(Input::Character('u')), 80, 80);

        // Check that the hunk is back
        let staged_diff_after_undo = OsCommand::new("git")
            .arg("diff")
            .arg("--staged")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get staged diff");
        let staged_diff_str_after_undo = String::from_utf8_lossy(&staged_diff_after_undo.stdout);
        assert!(staged_diff_str_after_undo.contains("modified line 16"));
        assert!(staged_diff_str_after_undo.contains("modified line 3"));

        // Cleanup
        std::fs::remove_dir_all(&repo_path).unwrap();
    }

    #[test]
    fn test_keydown_stops_at_last_line() {
        let mut state = create_state_with_files(1); // 1 file
        // Staged changes (0), file_0 (1), commit (2), prev_commit (3)
        let max_y = 30;
        let max_x = 80;

        // Cursor starts on the first file
        assert_eq!(state.file_cursor, 1);

        // KeyDown to commit line
        handle_navigation(&mut state, Input::KeyDown, max_y, max_x);
        assert_eq!(state.file_cursor, 2);

        // KeyDown to previous commit line
        handle_navigation(&mut state, Input::KeyDown, max_y, max_x);
        assert_eq!(state.file_cursor, 3);

        // KeyDown again, should not move
        handle_navigation(&mut state, Input::KeyDown, max_y, max_x);
        assert_eq!(state.file_cursor, 3);
    }

    #[test]
    fn test_stage_all_and_undo() {
        // Setup a temporary git repository
        let temp_dir = std::env::temp_dir().join("test_repo_for_stage_all");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();
        let repo_path = temp_dir;

        OsCommand::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to init git repo");
        OsCommand::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to set git user.name");
        OsCommand::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to set git user.email");

        // Create and commit a file
        let committed_file = repo_path.join("committed.txt");
        std::fs::write(&committed_file, "initial content").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to git add");
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to git commit");

        // Modify the committed file
        std::fs::write(&committed_file, "modified content").unwrap();

        // Create a new untracked file
        let untracked_file = repo_path.join("untracked.txt");
        std::fs::write(&untracked_file, "new file").unwrap();

        // Create app state
        let files = crate::git::get_unstaged_diff(&repo_path);
        let state = AppState::new(repo_path.clone(), files);

        // Simulate pressing 'R'
        let updated_state = update_state(state, Some(Input::Character('R')), 80, 80);

        // Check that both files are staged
        let status_output = OsCommand::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get git status");
        let status_str = String::from_utf8_lossy(&status_output.stdout);
        assert!(status_str.contains("M  committed.txt"));
        assert!(status_str.contains("A  untracked.txt"));

        // Simulate undo
        let _updated_state = update_state(updated_state, Some(Input::Character('u')), 80, 80);

        // Check that the original state is restored
        let status_output_after_undo = OsCommand::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get git status");
        let status_str_after_undo = String::from_utf8_lossy(&status_output_after_undo.stdout);
        assert!(status_str_after_undo.contains(" M committed.txt"));
        assert!(status_str_after_undo.contains("?? untracked.txt"));

        // Cleanup
        std::fs::remove_dir_all(&repo_path).unwrap();
    }

    #[test]
    fn test_tab_screen_switching_and_cursor_sync() {
        let mut state = create_state_with_files(0);
        state.files = vec![
            FileDiff {
                file_name: "staged_only.txt".to_string(),
                status: FileStatus::Modified,
                lines: vec![],
                hunks: vec![],
            },
            FileDiff {
                file_name: "common_file.txt".to_string(),
                status: FileStatus::Modified,
                lines: vec![],
                hunks: vec![],
            },
        ];
        state.unstaged_files = vec![
            FileDiff {
                file_name: "common_file.txt".to_string(),
                status: FileStatus::Modified,
                lines: vec![],
                hunks: vec![],
            },
            FileDiff {
                file_name: "unstaged_only.txt".to_string(),
                status: FileStatus::Modified,
                lines: vec![],
                hunks: vec![],
            },
        ];
        state.untracked_files = vec!["untracked_file.txt".to_string()];

        // --- Switch from Main to Unstaged (with file sync) ---
        state.screen = Screen::Main;
        state.file_cursor = 2; // "common_file.txt"

        let state = update_state(state, Some(Input::Character('\t')), 30, 80);
        assert_eq!(state.screen, Screen::Unstaged);
        assert_eq!(state.unstaged_cursor, 1); // "common_file.txt"

        // --- Switch from Unstaged to Main (with file sync) ---
        let mut state = update_state(state, Some(Input::Character('\t')), 30, 80);
        assert_eq!(state.screen, Screen::Main);
        assert_eq!(state.file_cursor, 2); // "common_file.txt"

        // --- Switch from Main to Unstaged (untracked file) ---
        state.files.push(FileDiff {
            file_name: "untracked_file.txt".to_string(),
            status: FileStatus::Added,
            lines: vec![],
            hunks: vec![],
        });
        state.file_cursor = 3; // "untracked_file.txt"
        let mut state = update_state(state, Some(Input::Character('\t')), 30, 80);
        assert_eq!(state.screen, Screen::Unstaged);
        // unstaged_files(2) + untracked_files(1) + headers(2) = 5 total
        // unstaged_cursor = unstaged_files.len() + index + 2
        // unstaged_cursor = 2 + 0 + 2 = 4
        assert_eq!(state.unstaged_cursor, 4);

        // --- Switch from Unstaged to Main (no sync) ---
        state.unstaged_cursor = 2; // "unstaged_only.txt"
        let state = update_state(state, Some(Input::Character('\t')), 30, 80);
        assert_eq!(state.screen, Screen::Main);
        assert_eq!(state.file_cursor, 1); // Reset to default
    }

    #[test]
    fn test_open_editor_main_view_no_line() {
        let mut state = create_state_with_files(1);
        state.file_cursor = 1;
        state.is_diff_cursor_active = false;
        external_command::mock::clear_calls();
        let repo_path = state.repo_path.clone();

        let _ = update_state(state, Some(Input::Character('e')), 80, 80);

        let calls = external_command::mock::get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            (
                repo_path.join("file_0.txt").to_str().unwrap().to_string(),
                None
            )
        );
    }

    #[test]
    fn test_open_editor_main_view_with_line() {
        let mut state = create_test_state(10, 1, 5, 0); // file_cursor=1, line_cursor=5
        state.is_diff_cursor_active = true;
        let mut file = create_test_file_diff();
        file.file_name = "test_file.rs".to_string();
        state.files = vec![file];
        external_command::mock::clear_calls();
        let repo_path = state.repo_path.clone();

        let _ = update_state(state, Some(Input::Character('e')), 80, 80);

        let calls = external_command::mock::get_calls();
        assert_eq!(calls.len(), 1);
        // line_cursor is 5, which is "+line 3 new" -> new_line_num 3
        assert_eq!(
            calls[0],
            (
                repo_path.join("test_file.rs").to_str().unwrap().to_string(),
                Some(3)
            )
        );
    }

    #[test]
    fn test_open_editor_unstaged_view() {
        let mut state = create_state_with_files(0);
        let mut file = create_test_file_diff();
        file.file_name = "unstaged_file.txt".to_string();
        state.unstaged_files = vec![file];
        state.screen = Screen::Unstaged;
        state.unstaged_cursor = 1; // Select the file
        state.line_cursor = 4; // "+line 2 new" -> new_line_num 2
        external_command::mock::clear_calls();
        let repo_path = state.repo_path.clone();

        let _ = update_state(state, Some(Input::Character('e')), 80, 80);

        let calls = external_command::mock::get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            (
                repo_path
                    .join("unstaged_file.txt")
                    .to_str()
                    .unwrap()
                    .to_string(),
                Some(2)
            )
        );
    }

    #[test]
    fn test_open_editor_untracked_file() {
        let mut state = create_state_with_files(0);
        state.untracked_files = vec!["untracked.txt".to_string()];
        state.screen = Screen::Unstaged;
        state.unstaged_cursor = 2; // [Unstaged header, Untracked header, untracked.txt]
        external_command::mock::clear_calls();
        let repo_path = state.repo_path.clone();

        let _ = update_state(state, Some(Input::Character('e')), 80, 80);

        let calls = external_command::mock::get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            (
                repo_path
                    .join("untracked.txt")
                    .to_str()
                    .unwrap()
                    .to_string(),
                None
            )
        );
    }

    #[test]
    fn test_unstage_all() {
        let repo_path = setup_temp_repo();
        // Create a committed file
        std::fs::write(repo_path.join("committed.txt"), "a\n").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&repo_path)
            .output()
            .unwrap();
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("i")
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create a modified file and a new file, then stage them
        std::fs::write(repo_path.join("committed.txt"), "b\n").unwrap();
        std::fs::write(repo_path.join("new.txt"), "c\n").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let files = crate::git::get_diff(repo_path.clone());
        let mut state = AppState::new(repo_path.clone(), files);
        state.file_cursor = 0; // Select "Staged changes" header

        // Unstage all
        let state = update_state(state, Some(Input::Character('\n')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(
            status.contains(" M committed.txt"),
            "Should be unstaged modified"
        );
        assert!(status.contains("?? new.txt"), "Should be untracked");

        // Undo
        let state = update_state(state, Some(Input::Character('u')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(
            status.contains("M  committed.txt"),
            "Should be staged modified"
        );
        assert!(status.contains("A  new.txt"), "Should be staged new");

        // Redo
        let _ = update_state(state, Some(Input::Character('r')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(status.contains(" M committed.txt"));
        assert!(status.contains("?? new.txt"));

        std::fs::remove_dir_all(&repo_path).unwrap();
    }

    #[test]
    fn test_stage_unstaged() {
        let repo_path = setup_temp_repo();
        std::fs::write(repo_path.join("file1.txt"), "a").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&repo_path)
            .output()
            .unwrap();
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("i")
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // One modified, one untracked
        std::fs::write(repo_path.join("file1.txt"), "b").unwrap();
        std::fs::write(repo_path.join("untracked.txt"), "c").unwrap();

        let mut state = AppState::new(repo_path.clone(), vec![]);
        state.refresh_diff();
        state.screen = Screen::Unstaged;
        state.unstaged_cursor = 0; // Select "Unstaged changes" header

        // Stage all unstaged
        let state = update_state(state, Some(Input::Character('\n')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(status.contains("M  file1.txt"), "file1 should be staged");
        assert!(
            status.contains("?? untracked.txt"),
            "untracked should remain untracked"
        );

        // Undo
        let state = update_state(state, Some(Input::Character('u')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(
            status.contains(" M file1.txt"),
            "file1 should be unstaged again"
        );
        assert!(
            status.contains("?? untracked.txt"),
            "untracked should still be untracked"
        );

        // Redo
        let _ = update_state(state, Some(Input::Character('r')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(status.contains("M  file1.txt"));
        assert!(status.contains("?? untracked.txt"));

        std::fs::remove_dir_all(&repo_path).unwrap();
    }

    #[test]
    fn test_stage_untracked() {
        let repo_path = setup_temp_repo();
        std::fs::write(repo_path.join("untracked1.txt"), "a").unwrap();
        std::fs::write(repo_path.join("untracked2.txt"), "b").unwrap();

        // Add a modified file to ensure we only stage untracked
        std::fs::write(repo_path.join("modified.txt"), "c").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg("modified.txt")
            .current_dir(&repo_path)
            .output()
            .unwrap();
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("i")
            .current_dir(&repo_path)
            .output()
            .unwrap();
        std::fs::write(repo_path.join("modified.txt"), "d").unwrap();

        let mut state = AppState::new(repo_path.clone(), vec![]);
        state.refresh_diff();
        state.screen = Screen::Unstaged;
        state.unstaged_cursor = state.unstaged_files.len() + 1; // Select "Untracked files" header

        // Stage all untracked
        let state = update_state(state, Some(Input::Character('\n')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(
            status.contains("A  untracked1.txt"),
            "untracked1 should be staged"
        );
        assert!(
            status.contains("A  untracked2.txt"),
            "untracked2 should be staged"
        );
        assert!(
            status.contains(" M modified.txt"),
            "modified.txt should NOT be staged"
        );

        // Undo
        let state = update_state(state, Some(Input::Character('u')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(
            status.contains("?? untracked1.txt"),
            "untracked1 should be untracked again"
        );
        assert!(
            status.contains("?? untracked2.txt"),
            "untracked2 should be untracked again"
        );
        assert!(
            status.contains(" M modified.txt"),
            "modified.txt should be untouched"
        );

        // Redo
        let _ = update_state(state, Some(Input::Character('r')), 80, 80);
        let status = get_git_status(&repo_path);
        assert!(status.contains("A  untracked1.txt"));
        assert!(status.contains("A  untracked2.txt"));
        assert!(status.contains(" M modified.txt"));

        std::fs::remove_dir_all(&repo_path).unwrap();
    }
}
