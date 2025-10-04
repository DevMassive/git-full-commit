use crate::app_state::{Screen};
use crate::ui::diff_view::LINE_CONTENT_OFFSET;
use crate::ui::main_screen::ListItem;
use crate::ui::update::*;
use pancurses::Input;

use super::other::*;

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
fn test_diff_view_updates_on_navigation_to_commit() {
    let repo_path = setup_temp_repo();
    use std::process::Command as OsCommand;

    // 1. Create one commit
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
        .arg("first commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // 2. Create AppState. With my fix to AppState::new, selected_commit_files should be empty.
    let state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    assert!(
        state.selected_commit_files.is_empty(),
        "selected_commit_files should be empty on init"
    );

    // 3. Navigate down to CommitMessageInput
    let state_on_commit_input = update_state(state, Some(Input::KeyDown), 80, 80);
    assert!(
        state_on_commit_input.selected_commit_files.is_empty(),
        "Diff should still be empty"
    );

    // 4. Navigate down to the previous commit
    let state_after_down = update_state(state_on_commit_input, Some(Input::KeyDown), 80, 80);

    // 5. Assert that the diff has been loaded
    assert!(
        !state_after_down.selected_commit_files.is_empty(),
        "selected_commit_files should be populated after navigating"
    );
    assert_eq!(
        state_after_down.selected_commit_files[0].file_name,
        "file1.txt"
    );

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_keydown_stops_at_last_line() {
    let state = create_state_with_files(1); // 1 file
    // Staged changes (0), file_0 (1), commit (2), prev_commit (3)
    let max_y = 30;
    let max_x = 80;
    assert_eq!(state.main_screen.list_items.len(), 3);

    // Cursor starts on the first file
    assert!(matches!(state.current_main_item(), Some(ListItem::File(_))));

    // KeyDown to commit line
    let state = update_state(state, Some(Input::KeyDown), max_y, max_x);
    assert_eq!(state.main_screen.file_cursor, 2);
    assert!(matches!(
        state.current_main_item(),
        Some(ListItem::CommitMessageInput)
    ));

    // KeyDown again, should not move
    let state = update_state(state, Some(Input::KeyDown), max_y, max_x);
    assert_eq!(state.main_screen.file_cursor, 2);
}

#[test]
fn test_tab_screen_switching_and_cursor_sync() {
    use crate::git::{FileDiff, FileStatus};
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