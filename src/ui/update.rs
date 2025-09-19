use crate::app_state::AppState;
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, DiscardHunkCommand, IgnoreFileCommand,
    RemoveFileCommand, UnstageFileCommand,
};
use crate::commit_storage;
use crate::git;
use crate::ui::commit_view;
use crate::ui::diff_view::LINE_CONTENT_OFFSET;
use crate::ui::scroll;
use pancurses::Input;
#[cfg(not(test))]
use pancurses::curs_set;

use crate::git_patch;

fn unstage_line(state: &mut AppState, max_y: i32) {
    if let Some(file) = state.current_file() {
        let line_index = state.line_cursor;
        if let Some(patch) = git_patch::create_unstage_line_patch(file, line_index) {
            let command = Box::new(ApplyPatchCommand {
                repo_path: state.repo_path.clone(),
                patch,
            });
            let old_line_cursor = state.line_cursor;
            state.execute_and_refresh(command);

            if let Some(file) = state.current_file() {
                state.line_cursor = old_line_cursor.min(file.lines.len().saturating_sub(1));
                let header_height = state.files.len() + 3;
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
        Input::Character('\u{3}') | Input::Character('Q') => {
            let _ = commit_storage::save_commit_message(&state.repo_path, &state.commit_message);
            state.running = false;
        }
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
                    let command = Box::new(IgnoreFileCommand {
                        repo_path: state.repo_path.clone(),
                        file_name: file.file_name.clone(),
                    });
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
                        let command = Box::new(DiscardHunkCommand {
                            repo_path: state.repo_path.clone(),
                            patch,
                        });
                        state.execute_and_refresh(command);
                    }
                }
            } else if let Some(file) = state.current_file().cloned() {
                let patch = git::get_file_diff_patch(&state.repo_path, &file.file_name)
                    .expect("Failed to get diff for file.");
                let command: Box<dyn crate::command::Command> =
                    if file.status == git::FileStatus::Added {
                        Box::new(RemoveFileCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                            patch,
                        })
                    } else {
                        Box::new(CheckoutFileCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                            patch,
                        })
                    };
                state.execute_and_refresh(command);
            }
        }
        Input::Character('\n') => {
            if let Some(file) = state.current_file().cloned() {
                let line_index = state.line_cursor;
                if let Some(hunk) = git_patch::find_hunk(&file, line_index) {
                    let patch = git_patch::create_unstage_hunk_patch(&file, hunk);
                    let command = Box::new(ApplyPatchCommand {
                        repo_path: state.repo_path.clone(),
                        patch,
                    });
                    state.execute_and_refresh(command);
                } else {
                    let command = Box::new(UnstageFileCommand {
                        repo_path: state.repo_path.clone(),
                        file_name: file.file_name.clone(),
                    });
                    state.execute_and_refresh(command);
                }
            }
        }
        Input::Character('1') => unstage_line(state, max_y),
        Input::Character('u') => {
            state.command_history.undo();
            state.refresh_diff();
        }
        Input::Character('r') => {
            state.command_history.redo();
            state.refresh_diff();
        }
        Input::Character('R') => {
            git::add_all(&state.repo_path).expect("Failed to git add -A.");
            state.refresh_diff();
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

            let num_files = state.files.len();
            let file_list_total_items = num_files + 3;
            let file_list_height = (max_y as usize / 3).max(3).min(file_list_total_items);

            if state.file_cursor >= state.file_list_scroll + file_list_height {
                state.file_list_scroll = state.file_cursor - file_list_height + 1;
            }

            if state.file_cursor == state.files.len() + 1 {
                state.is_commit_mode = true;
                #[cfg(not(test))]
                curs_set(1);
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
            let lines_count = if state.file_cursor == 0 {
                state
                    .previous_commit_files
                    .iter()
                    .map(|f| f.lines.len())
                    .sum()
            } else {
                state.current_file().map_or(0, |f| f.lines.len())
            };

            if lines_count > 0 && state.line_cursor < lines_count.saturating_sub(1) {
                state.line_cursor += 1;
            }

            let header_height = state.files.len() + 3;
            let content_height = (max_y as usize).saturating_sub(header_height);
            let cursor_line = state.get_cursor_line_index();

            if cursor_line >= state.scroll + content_height {
                state.scroll = cursor_line - content_height + 1;
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
        _ => {
            if state.file_cursor == state.files.len() + 1 {
                state.is_commit_mode = true;
                #[cfg(not(test))]
                curs_set(1);
                commit_view::handle_commit_input(state, input);
            } else {
                scroll::handle_scroll(state, input, max_y);
            }
        }
    }
}

pub fn update_state(mut state: AppState, input: Option<Input>, max_y: i32, max_x: i32) -> AppState {
    if state.is_commit_mode {
        if let Some(input) = input {
            commit_view::handle_commit_input(&mut state, input);
        }
        return state;
    }

    if let Some(input) = input {
        if !handle_commands(&mut state, input, max_y) {
            handle_navigation(&mut state, input, max_y, max_x);
        }
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::git::{FileDiff, FileStatus, Hunk};
    use pancurses::Input;
    use std::path::PathBuf;
    use std::process::Command as OsCommand;

    fn create_state_with_files(num_files: usize) -> AppState {
        let files: Vec<FileDiff> = (0..num_files)
            .map(|i| FileDiff {
                file_name: format!("file_{}.txt", i),
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
    fn test_page_down_maintains_relative_cursor() {
        let initial_state = create_test_state(100, 1, 5, 0);
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y, 80);

        assert_eq!(
            final_state.scroll, content_height,
            "Scroll should move down by one page"
        );
        assert_eq!(
            final_state.line_cursor,
            5 + content_height,
            "Cursor should also move down by one page"
        );
    }

    #[test]
    fn test_page_down_at_end_stops_at_max_scroll() {
        let lines_count = 100;
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let max_scroll = lines_count - content_height; // 74
        let initial_state = create_test_state(lines_count, 1, 80, max_scroll);

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y, 80);

        assert_eq!(
            final_state.scroll, max_scroll,
            "Scroll should not change as it's at the end"
        );
        assert_eq!(
            final_state.line_cursor, 80,
            "Cursor should not move as scroll did not change"
        );
    }

    #[test]
    fn test_page_down_clamps_at_end() {
        let lines_count = 40;
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(lines_count, 1, 20, 0);
        let max_scroll = lines_count - content_height; // 14

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y, 80);

        assert_eq!(
            final_state.scroll, max_scroll,
            "Scroll should clamp to the max scroll position"
        );
        assert_eq!(
            final_state.line_cursor,
            20 + max_scroll,
            "Cursor should move by the amount scrolled"
        );
    }

    // --- Page Up Tests ---

    #[test]
    fn test_page_up_maintains_relative_cursor() {
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(100, 1, 60, 50);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

        assert_eq!(
            final_state.scroll,
            50 - content_height,
            "Scroll should move up by one page"
        );
        assert_eq!(
            final_state.line_cursor,
            60 - content_height,
            "Cursor should also move up by one page"
        );
    }

    #[test]
    fn test_page_up_stops_at_top() {
        let max_y = 30;
        let _content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(100, 1, 20, 15);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

        assert_eq!(final_state.scroll, 0, "Scroll should clamp at the top");
        assert_eq!(
            final_state.line_cursor,
            20 - 15,
            "Cursor should move by the amount scrolled"
        );
    }

    #[test]
    fn test_page_up_at_top_does_nothing() {
        let max_y = 30;
        let _content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(100, 1, 10, 0);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

        assert_eq!(final_state.scroll, 0, "Scroll should not change");
        assert_eq!(final_state.line_cursor, 10, "Cursor should not change");
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
        updated_state.command_history.undo();
        updated_state.refresh_diff();

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

        // Simulate undo
        updated_state.command_history.undo();
        updated_state.refresh_diff();

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
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let content_height = (max_y as usize).saturating_sub(1 + 3);
        let scroll_amount = (content_height / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 10, 5);

        let final_state = update_state(initial_state, Some(Input::Character('\u{4}')), max_y, 80);

        let expected_scroll = 5 + scroll_amount;
        assert_eq!(final_state.scroll, expected_scroll);
        assert_eq!(final_state.line_cursor, 10 + scroll_amount);
    }

    #[test]
    fn test_half_page_down_and_scroll() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let content_height = (max_y as usize).saturating_sub(1 + 3);
        let scroll_amount = (content_height / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 25, 0);

        let final_state = update_state(initial_state, Some(Input::Character('\u{4}')), max_y, 80);

        assert_eq!(final_state.line_cursor, 25 + scroll_amount);
        assert_eq!(final_state.scroll, 13);
    }

    #[test]
    fn test_half_page_up() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let content_height = (max_y as usize).saturating_sub(1 + 3);
        let scroll_amount = (content_height / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 20, 15);

        let final_state = update_state(initial_state, Some(Input::Character('\u{15}')), max_y, 80);

        assert_eq!(final_state.line_cursor, 20 - scroll_amount);
        assert_eq!(final_state.scroll, 2);
    }

    #[test]
    fn test_half_page_up_and_scroll() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let _scroll_amount = ((max_y as usize).saturating_sub(1 + 3) / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 10, 10);

        let final_state = update_state(initial_state, Some(Input::Character('\u{15}')), max_y, 80);

        assert_eq!(final_state.line_cursor, 0); // 10 - 13 saturates at 0
        assert_eq!(final_state.scroll, 0); // 10 - 13 saturates at 0
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
        let initial_content = (1..=20).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
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
        let mut updated_state = update_state(state, Some(Input::Character('!')), 80, 80);

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
        assert!(working_diff_str.is_empty(), "Working directory should be clean");

        // Simulate undo
        updated_state.command_history.undo();
        updated_state.refresh_diff();

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
}
