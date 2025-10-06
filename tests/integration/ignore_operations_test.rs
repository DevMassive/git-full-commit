use crate::git_test::common::TestRepo;
use git_full_commit::app_state::{AppState, FocusedPane};
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;
use std::fs;

#[test]
fn test_ignore_staged_file() {
    let repo = TestRepo::new();
    let file_to_ignore = "a.txt";
    repo.create_file(file_to_ignore, "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.files.len(), 1);

    app_state.main_screen.file_cursor = 1;

    app_state = update_state(app_state, Some(Input::Character('i')), 80, 80);

    let gitignore_content = fs::read_to_string(repo.path.join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(file_to_ignore));

    // .gitignore is staged
    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.files[0].file_name, ".gitignore");

    // original file is no longer tracked by git
    let untracked = git::get_untracked_files(&repo.path).unwrap();
    assert!(!untracked.contains(&file_to_ignore.to_string()));
}

#[test]
fn test_ignore_untracked_file() {
    let repo = TestRepo::new();
    let file_to_ignore = "a.txt";
    repo.create_file(file_to_ignore, "hello");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.unstaged_pane.untracked_files.len(), 1);

    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.focused_pane, FocusedPane::Unstaged);

    app_state.unstaged_pane.cursor = 2;

    app_state = update_state(app_state, Some(Input::Character('i')), 80, 80);

    let gitignore_content = fs::read_to_string(repo.path.join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(file_to_ignore));

    // .gitignore is staged
    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.files[0].file_name, ".gitignore");

    assert!(app_state.unstaged_pane.untracked_files.is_empty());
}
