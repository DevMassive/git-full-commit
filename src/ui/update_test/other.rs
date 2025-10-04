use crate::app_state::AppState;
use crate::git::{FileDiff, FileStatus, Hunk};
use crate::ui::update::*;
use pancurses::Input;
use std::path::PathBuf;
use std::process::Command as OsCommand;

// Helper function to create a temporary git repository for testing
pub fn setup_temp_repo() -> PathBuf {
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

pub fn get_git_status(repo_path: &PathBuf) -> String {
    let output = OsCommand::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(repo_path)
        .output()
        .expect("Failed to get git status");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn create_test_file_diff() -> FileDiff {
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
        old_file_name: "test.txt".to_string(),
        hunks,
        lines,
        status: FileStatus::Modified,
    }
}

pub fn create_state_with_files(num_files: usize) -> AppState {
    let files: Vec<FileDiff> = (0..num_files)
        .map(|i| FileDiff {
            file_name: format!("file_{i}.txt"),
            old_file_name: format!("file_{i}.txt"),
            status: FileStatus::Modified,
            lines: vec![],
            hunks: vec![],
        })
        .collect();

    let mut state = AppState::new(PathBuf::from("/tmp"), files);
    state.selected_commit_files = vec![];
    state
}

pub fn create_test_state(
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
            old_file_name: "test_file.rs".to_string(),
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
    state.main_screen.file_cursor = file_cursor;
    state.main_screen.line_cursor = line_cursor;
    state.main_screen.diff_scroll = scroll;
    // Mock previous commit files to avoid git command execution in tests
    state.selected_commit_files = vec![];
    state
}

#[test]
fn test_q_behavior_with_active_diff_cursor() {
    let mut state = create_test_state(10, 1, 0, 0);
    state.main_screen.is_diff_cursor_active = true;

    // First 'q' should only deactivate the cursor
    let state_after_first_q = update_state(state, Some(Input::Character('q')), 30, 80);
    assert!(
        state_after_first_q.running,
        "App should still be running after first 'q'"
    );
    assert!(
        !state_after_first_q.main_screen.is_diff_cursor_active,
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