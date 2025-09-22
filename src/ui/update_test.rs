use crate::app_state::{AppState, Screen};
use crate::git::{FileDiff, FileStatus, Hunk};
use crate::ui::diff_view::LINE_CONTENT_OFFSET;
use crate::ui::update::*;
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
        old_file_name: "test.txt".to_string(),
        hunks,
        lines,
        status: FileStatus::Modified,
    }
}

fn create_state_with_files(num_files: usize) -> AppState {
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

#[test]
fn test_file_list_scrolling() {
    let mut state = create_state_with_files(50);
    let max_y = 30; // file_list_height = 10

    // --- Scroll down ---
    // Move to cursor 9, scroll should be 0
    for _ in 0..8 {
        state = update_state(state, Some(Input::KeyDown), max_y, 80);
    }
    assert_eq!(state.main_screen.file_cursor, 9);
    assert_eq!(state.main_screen.file_list_scroll, 0);

    // Move to cursor 10, scroll should be 1
    state = update_state(state, Some(Input::KeyDown), max_y, 80);
    assert_eq!(state.main_screen.file_cursor, 10);
    assert_eq!(state.main_screen.file_list_scroll, 1);

    // Move to cursor 20, scroll should be 11
    for _ in 0..10 {
        state = update_state(state, Some(Input::KeyDown), max_y, 80);
    }
    assert_eq!(state.main_screen.file_cursor, 20);
    assert_eq!(state.main_screen.file_list_scroll, 11);

    // --- Scroll up ---
    // Move to cursor 11, scroll should be 11
    for _ in 0..9 {
        state = update_state(state, Some(Input::KeyUp), max_y, 80);
    }
    assert_eq!(state.main_screen.file_cursor, 11);
    assert_eq!(state.main_screen.file_list_scroll, 11);

    // Move to cursor 10, scroll should be 10
    state = update_state(state, Some(Input::KeyUp), max_y, 80);
    assert_eq!(state.main_screen.file_cursor, 10);
    assert_eq!(state.main_screen.file_list_scroll, 10);

    // Move to cursor 0, scroll should be 0
    for _ in 0..10 {
        state = update_state(state, Some(Input::KeyUp), max_y, 80);
    }
    assert_eq!(state.main_screen.file_cursor, 0);
    assert_eq!(state.main_screen.file_list_scroll, 0);
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
        final_state.main_screen.line_cursor, expected_cursor,
        "Cursor should move down by one page"
    );
    // Scroll should jump by a page, not just follow the cursor
    assert_eq!(
        final_state.main_screen.diff_scroll, content_height,
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
        final_state.main_screen.line_cursor, 99,
        "Cursor should move to the last line"
    );
    assert_eq!(
        final_state.main_screen.diff_scroll,
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
    assert_eq!(final_state.main_screen.line_cursor, expected_cursor);

    assert_eq!(final_state.main_screen.diff_scroll, content_height); // 25
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
        final_state.main_screen.line_cursor, expected_cursor,
        "Cursor should move up by one page"
    );
    assert_eq!(
        final_state.main_screen.diff_scroll,
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
        final_state.main_screen.line_cursor,
        20_usize.saturating_sub(content_height), // 0
        "Cursor should move up by one page or saturate at 0"
    );
    assert_eq!(
        final_state.main_screen.diff_scroll, 0,
        "Scroll should clamp at the top"
    );
}

#[test]
fn test_page_up_at_top_does_nothing() {
    let max_y = 30;
    let _content_height = (max_y as usize).saturating_sub(1 + 4);
    let initial_state = create_test_state(100, 1, 10, 0);

    let final_state = update_state(initial_state, Some(Input::Character('b')), max_y, 80);

    assert_eq!(
        final_state.main_screen.diff_scroll, 0,
        "Scroll should not change"
    );
    assert_eq!(
        final_state.main_screen.line_cursor, 0,
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
    state.main_screen.file_cursor = 1; // Select the file

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
    updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

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
    updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

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
    assert_eq!(final_state.main_screen.line_cursor, expected_cursor);
    // Cursor is at 22, scroll is 5, content_height is 25. 22 < 5 + 25. No scroll.
    assert_eq!(final_state.main_screen.diff_scroll, 5);
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
    assert_eq!(final_state.main_screen.line_cursor, expected_cursor);
    // Cursor is at 32, scroll is 0, content_height is 25. 32 >= 0 + 25. Scroll.
    let expected_scroll = scroll_amount;
    assert_eq!(final_state.main_screen.diff_scroll, expected_scroll);
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
    assert_eq!(final_state.main_screen.line_cursor, expected_cursor);
    // Cursor is at 8, scroll is 15. 8 < 15. Scroll.
    let expected_scroll = 15 - scroll_amount; // 3
    assert_eq!(final_state.main_screen.diff_scroll, expected_scroll.max(0));
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
    assert_eq!(final_state.main_screen.line_cursor, expected_cursor);
    // Cursor is at 0, scroll is 10. 0 < 10. Scroll.
    let expected_scroll = 10_usize.saturating_sub(scroll_amount); // 0
    assert_eq!(final_state.main_screen.diff_scroll, expected_scroll);
}

#[test]
fn test_horizontal_scroll() {
    let mut state = create_test_state(10, 1, 0, 0);
    assert_eq!(state.main_screen.horizontal_scroll, 0);
    let max_x = 80;
    let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);

    // Scroll right
    state = update_state(state, Some(Input::KeyRight), 30, max_x);
    assert_eq!(state.main_screen.horizontal_scroll, scroll_amount);
    state = update_state(state, Some(Input::KeyRight), 30, max_x);
    assert_eq!(state.main_screen.horizontal_scroll, scroll_amount * 2);

    // Scroll left
    state = update_state(state, Some(Input::KeyLeft), 30, max_x);
    assert_eq!(state.main_screen.horizontal_scroll, scroll_amount);
    state = update_state(state, Some(Input::KeyLeft), 30, max_x);
    assert_eq!(state.main_screen.horizontal_scroll, 0);

    // Scroll left at 0 should not change
    state = update_state(state, Some(Input::KeyLeft), 30, max_x);
    assert_eq!(state.main_screen.horizontal_scroll, 0);
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
    state.main_screen.file_cursor = 1; // Select the file
    state.main_screen.is_diff_cursor_active = true;
    // Move cursor to the second hunk (around line 16)
    // The diff output will have headers and context lines, so we need to estimate the line number
    let line_in_diff = state.files[0]
        .lines
        .iter()
        .position(|l| l.contains("modified line 16"))
        .unwrap_or(15);
    state.main_screen.line_cursor = line_in_diff;

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
    let _updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

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
    let state = create_state_with_files(1); // 1 file
    // Staged changes (0), file_0 (1), commit (2), prev_commit (3)
    let max_y = 30;
    let max_x = 80;

    // Cursor starts on the first file
    assert_eq!(state.main_screen.file_cursor, 1);

    // KeyDown to commit line
    let state = update_state(state, Some(Input::KeyDown), max_y, max_x);
    assert_eq!(state.main_screen.file_cursor, 2);

    // KeyDown to previous commit line
    let state = update_state(state, Some(Input::KeyDown), max_y, max_x);
    assert_eq!(state.main_screen.file_cursor, 3);

    // KeyDown again, should not move
    let state = update_state(state, Some(Input::KeyDown), max_y, max_x);
    assert_eq!(state.main_screen.file_cursor, 3);
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
    let _updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

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
    let staged_file1 = FileDiff {
        file_name: "staged_only.txt".to_string(),
        old_file_name: "staged_only.txt".to_string(),
        status: FileStatus::Modified,
        lines: vec![],
        hunks: vec![],
    };
    let staged_file2 = FileDiff {
        file_name: "common_file.txt".to_string(),
        old_file_name: "common_file.txt".to_string(),
        status: FileStatus::Modified,
        lines: vec![],
        hunks: vec![],
    };
    state.files = vec![staged_file1.clone(), staged_file2.clone()];

    let unstaged_file1 = FileDiff {
        file_name: "common_file.txt".to_string(),
        old_file_name: "common_file.txt".to_string(),
        status: FileStatus::Modified,
        lines: vec![],
        hunks: vec![],
    };
    let unstaged_file2 = FileDiff {
        file_name: "unstaged_only.txt".to_string(),
        old_file_name: "unstaged_only.txt".to_string(),
        status: FileStatus::Modified,
        lines: vec![],
        hunks: vec![],
    };
    state.unstaged_screen.unstaged_files = vec![unstaged_file1.clone(), unstaged_file2.clone()];
    state.unstaged_screen.untracked_files = vec!["untracked_file.txt".to_string()];
    state.main_screen.has_unstaged_changes = true;

    // Manually build list_items for this test
    state.main_screen.list_items = vec![
        crate::ui::main_screen::ListItem::StagedChangesHeader,
        crate::ui::main_screen::ListItem::File(staged_file1.clone()),
        crate::ui::main_screen::ListItem::File(staged_file2.clone()),
        crate::ui::main_screen::ListItem::CommitMessageInput,
        crate::ui::main_screen::ListItem::PreviousCommitInfo {
            hash: String::new(),
            message: String::new(),
            is_on_remote: false,
        },
    ];

    // --- Switch from Main to Unstaged (with file sync) ---
    state.screen = Screen::Main;
    state.main_screen.file_cursor = 2; // "common_file.txt" (index 2 in list_items)

    let state = update_state(state, Some(Input::Character('\t')), 30, 80);
    assert_eq!(state.screen, Screen::Unstaged);
    assert_eq!(state.unstaged_screen.unstaged_cursor, 1); // "common_file.txt" (index 1 in unstaged_files)

    // --- Switch from Unstaged to Main (with file sync) ---
    let mut state = update_state(state, Some(Input::Character('\t')), 30, 80);
    assert_eq!(state.screen, Screen::Main);
    assert_eq!(state.main_screen.file_cursor, 2); // "common_file.txt"

    // --- Switch from Main to Unstaged (untracked file) ---
    let untracked_file_diff = FileDiff {
        file_name: "untracked_file.txt".to_string(),
        old_file_name: "untracked_file.txt".to_string(),
        status: FileStatus::Added,
        lines: vec![],
        hunks: vec![],
    };
    state.files.push(untracked_file_diff.clone());
    state
        .main_screen
        .list_items
        .push(crate::ui::main_screen::ListItem::File(
            untracked_file_diff.clone(),
        )); // Add to list_items
    state.main_screen.file_cursor = 5; // "untracked_file.txt" (index 5 in list_items)

    let mut state = update_state(state, Some(Input::Character('\t')), 30, 80);
    assert_eq!(state.screen, Screen::Unstaged);
    // unstaged_files(2) + untracked_files(1) + headers(2) = 5 total
    // unstaged_cursor = unstaged_files.len() + index + 2
    // unstaged_cursor = 2 + 0 + 2 = 4
    assert_eq!(state.unstaged_screen.unstaged_cursor, 4);

    // --- Switch from Unstaged to Main (no sync) ---
    state.unstaged_screen.unstaged_cursor = 2; // "unstaged_only.txt"
    let state = update_state(state, Some(Input::Character('\t')), 30, 80);
    assert_eq!(state.screen, Screen::Main);
    assert_eq!(state.main_screen.file_cursor, 5); // Unchanged
}

#[test]
fn test_open_editor_main_view_no_line() {
    let mut state = create_state_with_files(1);
    state.main_screen.file_cursor = 1;
    state.main_screen.is_diff_cursor_active = false;
    let repo_path = state.repo_path.clone();

    let updated_state = update_state(state, Some(Input::Character('e')), 80, 80);

    assert!(updated_state.editor_request.is_some());
    let request = updated_state.editor_request.unwrap();
    assert_eq!(
        request.file_path,
        repo_path.join("file_0.txt").to_str().unwrap()
    );
    assert_eq!(request.line_number, None);
}

#[test]
fn test_open_editor_main_view_with_line() {
    let mut state = create_test_state(0, 0, 5, 0); // Start with no files
    state.main_screen.is_diff_cursor_active = true;
    let mut file = create_test_file_diff();
    file.file_name = "test_file.rs".to_string();

    // Manually build list_items for this test
    state.main_screen.list_items = vec![
        crate::ui::main_screen::ListItem::StagedChangesHeader,
        crate::ui::main_screen::ListItem::File(file.clone()),
        crate::ui::main_screen::ListItem::CommitMessageInput,
        crate::ui::main_screen::ListItem::PreviousCommitInfo {
            hash: String::new(),
            message: String::new(),
            is_on_remote: false,
        },
    ];
    state.main_screen.file_cursor = 1; // Select the file
    state.files = vec![file]; // Keep this for current_file() to work in the test context if it's still used elsewhere.

    let repo_path = state.repo_path.clone();

    let updated_state = update_state(state, Some(Input::Character('e')), 80, 80);

    assert!(updated_state.editor_request.is_some());
    let request = updated_state.editor_request.unwrap();
    assert_eq!(
        request.file_path,
        repo_path.join("test_file.rs").to_str().unwrap()
    );
    assert_eq!(request.line_number, Some(3));
}

#[test]
fn test_open_editor_unstaged_screen() {
    let mut state = create_state_with_files(0);
    let mut file = create_test_file_diff();
    file.file_name = "unstaged_file.txt".to_string();
    state.unstaged_screen.unstaged_files = vec![file.clone()];
    state.unstaged_screen.list_items = AppState::build_unstaged_screen_list_items(
        &state.unstaged_screen.unstaged_files,
        &state.unstaged_screen.untracked_files,
    );
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 1; // Select the file
    state.main_screen.line_cursor = 4; // "+line 2 new" -> new_line_num 2
    let repo_path = state.repo_path.clone();

    let updated_state = update_state(state, Some(Input::Character('e')), 80, 80);

    assert!(updated_state.editor_request.is_some());
    let request = updated_state.editor_request.unwrap();
    assert_eq!(
        request.file_path,
        repo_path.join("unstaged_file.txt").to_str().unwrap()
    );
    assert_eq!(request.line_number, Some(2));
}

#[test]
fn test_open_editor_untracked_file() {
    let mut state = create_state_with_files(0);
    state.unstaged_screen.untracked_files = vec!["untracked.txt".to_string()];
    state.unstaged_screen.list_items = AppState::build_unstaged_screen_list_items(
        &state.unstaged_screen.unstaged_files,
        &state.unstaged_screen.untracked_files,
    );
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2; // [Unstaged header, Untracked header, untracked.txt]
    let repo_path = state.repo_path.clone();

    let updated_state = update_state(state, Some(Input::Character('e')), 80, 80);

    assert!(updated_state.editor_request.is_some());
    let request = updated_state.editor_request.unwrap();
    assert_eq!(
        request.file_path,
        repo_path.join("untracked.txt").to_str().unwrap()
    );
    assert_eq!(request.line_number, None);
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
    state.main_screen.file_cursor = 0; // Select "Staged changes" header

    // Unstage all
    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains(" M committed.txt"),
        "Should be unstaged modified"
    );
    assert!(status.contains("?? new.txt"), "Should be untracked");

    // Undo
    // Ensure file cursor is on "Staged changes" header
    assert_eq!(state.main_screen.file_cursor, 0);
    let state = update_state(state, Some(Input::Character('<')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains("M  committed.txt"),
        "Should be staged modified"
    );
    assert!(status.contains("A  new.txt"), "Should be staged new");

    // Redo
    let _ = update_state(state, Some(Input::Character('>')), 80, 80);
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
    state.unstaged_screen.unstaged_cursor = 0; // Select "Unstaged changes" header

    // Stage all unstaged
    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(status.contains("M  file1.txt"), "file1 should be staged");
    assert!(
        status.contains("?? untracked.txt"),
        "untracked should remain untracked"
    );

    // Undo
    let state = update_state(state, Some(Input::Character('<')), 80, 80);
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
    let _ = update_state(state, Some(Input::Character('>')), 80, 80);
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
    state.unstaged_screen.unstaged_cursor = state.unstaged_screen.unstaged_files.len() + 1; // Select "Untracked files" header

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
    let state = update_state(state, Some(Input::Character('<')), 80, 80);
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
    let _ = update_state(state, Some(Input::Character('>')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(status.contains("A  untracked1.txt"));
    assert!(status.contains("A  untracked2.txt"));
    assert!(status.contains(" M modified.txt"));

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_discard_unstaged_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("test.txt");
    std::fs::write(&file_path, "line1\nline2\n").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::fs::write(&file_path, "line1\nMODIFIED\n").unwrap();

    let mut state = AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 1;

    let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.is_empty(),
        "Git status should be clean after discard"
    );
    assert!(
        state_after_discard
            .unstaged_screen
            .unstaged_files
            .is_empty()
    );

    let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
    let status_after_undo = get_git_status(&repo_path);
    assert!(
        status_after_undo.contains(" M test.txt"),
        "File should be modified again after undo"
    );
    assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);
    assert_eq!(
        state_after_undo.unstaged_screen.unstaged_files[0].file_name,
        "test.txt"
    );

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_discard_untracked_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("untracked.txt");
    std::fs::write(&file_path, "hello").unwrap();

    let mut state = AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2; // Header, Header, File

    let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
    assert!(!file_path.exists(), "File should be deleted");
    assert!(
        state_after_discard
            .unstaged_screen
            .untracked_files
            .is_empty()
    );

    let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
    assert!(file_path.exists(), "File should be restored after undo");
    assert_eq!(state_after_undo.unstaged_screen.untracked_files.len(), 1);
    assert_eq!(
        state_after_undo.unstaged_screen.untracked_files[0],
        "untracked.txt"
    );

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_discard_untracked_binary_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("binary.bin");
    std::fs::write(&file_path, b"hello\0world").unwrap();

    let mut state = AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2;

    let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
    assert!(file_path.exists(), "Binary file should not be deleted");
    assert_eq!(state_after_discard.unstaged_screen.untracked_files.len(), 1);

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_ignore_unstaged_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("test.txt");
    std::fs::write(&file_path, "initial").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::fs::write(&file_path, "modified").unwrap();

    let mut state = AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 1;

    let state_after_ignore = update_state(state, Some(Input::Character('i')), 80, 80);
    let gitignore_content =
        std::fs::read_to_string(repo_path.join(".gitignore")).unwrap_or_default();
    assert!(gitignore_content.contains("test.txt"));
    let status = get_git_status(&repo_path);
    assert!(status.contains("A  .gitignore"));
    assert!(state_after_ignore.unstaged_screen.unstaged_files.is_empty());

    let state_after_undo = update_state(state_after_ignore, Some(Input::Character('<')), 80, 80);
    let status_after_undo = get_git_status(&repo_path);
    assert!(!repo_path.join(".gitignore").exists());
    assert!(status_after_undo.contains(" M test.txt"));
    assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_ignore_untracked_file() {
    let repo_path = setup_temp_repo();
    std::fs::write(repo_path.join("untracked.txt"), "hello").unwrap();

    let mut state = AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2;

    let state_after_ignore = update_state(state, Some(Input::Character('i')), 80, 80);
    let gitignore_content =
        std::fs::read_to_string(repo_path.join(".gitignore")).unwrap_or_default();
    assert!(gitignore_content.contains("untracked.txt"));
    let status = get_git_status(&repo_path);
    assert!(status.contains("A  .gitignore"));
    assert!(!status.contains("untracked.txt"));
    assert!(
        state_after_ignore
            .unstaged_screen
            .untracked_files
            .is_empty()
    );

    let state_after_undo = update_state(state_after_ignore, Some(Input::Character('<')), 80, 80);
    let status_after_undo = get_git_status(&repo_path);
    assert!(!repo_path.join(".gitignore").exists());
    assert!(status_after_undo.contains("?? untracked.txt"));
    assert_eq!(state_after_undo.unstaged_screen.untracked_files.len(), 1);

    std::fs::remove_dir_all(&repo_path).unwrap();
}
