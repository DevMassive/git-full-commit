use crate::git_test::common::TestRepo;
use git_full_commit::app_state::{AppState, FocusedPane};
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;
use std::fs;

#[test]
fn test_discard_staged_file() {
    // Setup repo with a staged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial");
    repo.create_file("a.txt", "world");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.files.len(), 1);

    // Navigate to the file
    app_state.main_screen.file_cursor = 1;

    // Press '!' to discard
    app_state = update_state(app_state, Some(Input::Character('!')), 80, 80);

    assert!(app_state.files.is_empty());
    let status = repo.get_status();
    assert!(!status.contains("a.txt"));
}

#[test]
fn test_discard_staged_hunk() {
    let repo = TestRepo::new();
    let initial_content: String = (1..=10).map(|i| format!("line{i}\n")).collect();
    repo.create_file("a.txt", &initial_content);
    repo.add_all();
    repo.commit("initial");

    let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
    lines[0] = "changed1".to_string();
    lines[9] = "changed10".to_string();
    repo.create_file("a.txt", &lines.join("\n"));
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.files[0].hunks.len(), 2);

    app_state.main_screen.file_cursor = 1;
    let line_in_second_hunk = app_state.files[0]
        .lines
        .iter()
        .position(|l| l.contains("+changed10"))
        .unwrap();
    app_state.main_screen.line_cursor = line_in_second_hunk;
    app_state.main_screen.is_diff_cursor_active = true;

    app_state = update_state(app_state, Some(Input::Character('!')), 80, 80);

    assert_eq!(app_state.files[0].hunks.len(), 1);
    assert!(
        !app_state.files[0]
            .lines
            .iter()
            .any(|l| l.contains("changed10"))
    );

    let file_content = fs::read_to_string(repo.path.join("a.txt")).unwrap();
    assert!(!file_content.contains("changed10"));
    assert!(file_content.contains("changed1"));
}

#[test]
fn test_discard_unstaged_file() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial");
    repo.create_file("a.txt", "world");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.focused_pane, FocusedPane::Unstaged);

    app_state.unstaged_pane.cursor = 1;
    app_state = update_state(app_state, Some(Input::Character('!')), 80, 80);

    assert!(app_state.unstaged_pane.unstaged_files.is_empty());
    let status = repo.get_status();
    assert!(!status.contains("a.txt"));
}

#[test]
fn test_discard_untracked_file() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);

    app_state.unstaged_pane.cursor = 2;
    app_state = update_state(app_state, Some(Input::Character('!')), 80, 80);

    assert!(app_state.unstaged_pane.untracked_files.is_empty());
    let status = repo.get_status();
    assert!(!status.contains("a.txt"));
}
