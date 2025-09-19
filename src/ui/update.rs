use crate::app_state::AppState;
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, IgnoreFileCommand, RemoveFileCommand,
    UnstageFileCommand,
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
    if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
        if let Some(file) = state.files.get(state.file_cursor - 1) {
            let line_index = state.line_cursor;
            if let Some(patch) = git_patch::create_unstage_line_patch(file, line_index) {
                let command = Box::new(ApplyPatchCommand {
                    repo_path: state.repo_path.clone(),
                    patch,
                });
                let old_line_cursor = state.line_cursor;
                state.command_history.execute(command);
                state.refresh_diff();
                if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                    if let Some(file) = state.files.get(state.file_cursor - 1) {
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
        match input {
            Input::Character('\u{3}') | Input::Character('q') | Input::Character('Q') => {
                // Ctrl+C or Q or q
                let _ =
                    commit_storage::save_commit_message(&state.repo_path, &state.commit_message);
                state.running = false;
            }
            Input::Character('i') => {
                if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                    if let Some(file) = state.files.get(state.file_cursor - 1).cloned() {
                        if file.file_name == ".gitignore" {
                            return state;
                        }
                        let command = Box::new(IgnoreFileCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                        });
                        state.command_history.execute(command);
                        state.refresh_diff();
                    }
                }
            }
            Input::Character('!') => {
                if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                    if let Some(file) = state.files.get(state.file_cursor - 1).cloned() {
                        // Get the patch before doing anything
                        let patch = git::get_file_diff_patch(&state.repo_path, &file.file_name)
                            .expect("Failed to get diff for file.");

                        if file.status == git::FileStatus::Added {
                            let command = Box::new(RemoveFileCommand {
                                repo_path: state.repo_path.clone(),
                                file_name: file.file_name.clone(),
                                patch,
                            });
                            state.command_history.execute(command);
                        } else {
                            let command = Box::new(CheckoutFileCommand {
                                repo_path: state.repo_path.clone(),
                                file_name: file.file_name.clone(),
                                patch,
                            });
                            state.command_history.execute(command);
                        }
                        state.refresh_diff();
                    }
                }
            }
            Input::Character('\n') => {
                if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                    if let Some(file) = state.files.get(state.file_cursor - 1).cloned() {
                        let line_index = state.line_cursor;
                        if let Some(hunk) = git_patch::find_hunk(&file, line_index) {
                            let patch = git_patch::create_unstage_hunk_patch(&file, hunk);

                            let command = Box::new(ApplyPatchCommand {
                                repo_path: state.repo_path.clone(),
                                patch,
                            });
                            state.command_history.execute(command);
                            state.refresh_diff();
                        } else {
                            let command = Box::new(UnstageFileCommand {
                                repo_path: state.repo_path.clone(),
                                file_name: file.file_name.clone(),
                            });
                            state.command_history.execute(command);
                            state.refresh_diff();
                        }
                    }
                }
            }
            Input::Character('1') => {
                unstage_line(&mut state, max_y);
            }
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
            Input::KeyUp => {
                state.file_cursor = state.file_cursor.saturating_sub(1);
                state.scroll = 0;
                state.line_cursor = 0;
            }
            Input::KeyDown => {
                if state.file_cursor < state.files.len() + 1 {
                    state.file_cursor += 1;
                    state.scroll = 0;
                    state.line_cursor = 0;
                }

                if state.file_cursor == state.files.len() + 1 {
                    state.is_commit_mode = true;
                    #[cfg(not(test))]
                    curs_set(1);
                }
            }
            Input::Character('k') => {
                state.line_cursor = state.line_cursor.saturating_sub(1);
                let cursor_line = state.get_cursor_line_index();
                if cursor_line < state.scroll {
                    state.scroll = cursor_line;
                }
            }
            Input::Character('j') => {
                let lines_count = if state.file_cursor == 0 {
                    state
                        .previous_commit_files
                        .iter()
                        .map(|f| f.lines.len())
                        .sum()
                } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                    state
                        .files
                        .get(state.file_cursor - 1)
                        .map_or(0, |f| f.lines.len())
                } else {
                    0
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
                scroll::handle_scroll(&mut state, input, max_y);
            }
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
}
