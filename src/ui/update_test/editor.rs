use crate::app_state::{AppState, Screen};
use crate::ui::update::*;
use pancurses::Input;

use super::other::*;

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