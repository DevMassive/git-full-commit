use crate::git_test::common::TestRepo;
use git_full_commit::app_state::{AppState, FocusedPane};
use git_full_commit::git;
use git_full_commit::ui::main_screen::UnstagedListItem;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

#[test]
fn test_initial_screen_layout_and_state() {
    let repo = TestRepo::new();
    let files = git::get_diff(repo.path.clone());
    let app_state = AppState::new(repo.path, files);

    // Spec: Main Screen is the initial view.
    assert_eq!(app_state.focused_pane, FocusedPane::Main);

    // Spec: The cursor is positioned on the first item in the list.
    assert_eq!(app_state.main_screen.file_cursor, 0);

    // Spec: The Diff Cursor state is initially INACTIVE.
    assert!(!app_state.main_screen.is_diff_cursor_active);
}

#[test]
fn test_main_screen_list_navigation() {
    // Setup repo with a commit and a staged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial commit");
    repo.create_file("b.txt", "world");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path, files);
    
    // In AppState::new, cursor is placed on the first file if it exists.
    // list: [Header, File, Input, Commit], so cursor is at index 1.
    assert_eq!(app_state.main_screen.file_cursor, 1);

    // Navigate down
    app_state = update_state(app_state, Some(Input::KeyDown), 80, 80);
    assert_eq!(app_state.main_screen.file_cursor, 2);
    assert!(!app_state.main_screen.is_diff_cursor_active, "Diff cursor should be inactive after KeyDown");

    // Navigate down again
    app_state = update_state(app_state, Some(Input::KeyDown), 80, 80);
    assert_eq!(app_state.main_screen.file_cursor, 3);

    // Navigate up
    app_state = update_state(app_state, Some(Input::KeyUp), 80, 80);
    assert_eq!(app_state.main_screen.file_cursor, 2);
    assert!(!app_state.main_screen.is_diff_cursor_active, "Diff cursor should be inactive after KeyUp");
}

#[test]
fn test_main_screen_diff_navigation_activation() {
    // Setup repo with a staged file with multiple lines
    let repo = TestRepo::new();
    repo.create_file("a.txt", "line1\nline2\nline3");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path, files);

    // In AppState::new, cursor is placed on the first file if it exists.
    // list: [Header, File, Input], so cursor is at index 1.
    assert_eq!(app_state.main_screen.file_cursor, 1);
    assert!(!app_state.main_screen.is_diff_cursor_active);
    assert_eq!(app_state.main_screen.line_cursor, 0);

    // Press 'j' to activate diff cursor and move down
    app_state = update_state(app_state, Some(Input::Character('j')), 80, 80);
    assert!(app_state.main_screen.is_diff_cursor_active);
    assert_eq!(app_state.main_screen.file_cursor, 1, "File cursor should not change");
    assert_eq!(app_state.main_screen.line_cursor, 1, "Line cursor should move down by 1");

    // Press 'j' again
    app_state = update_state(app_state, Some(Input::Character('j')), 80, 80);
    assert_eq!(app_state.main_screen.line_cursor, 2, "Line cursor should move down by 1 again");

    // Press 'k' to move up in the diff
    app_state = update_state(app_state, Some(Input::Character('k')), 80, 80);
    assert!(app_state.main_screen.is_diff_cursor_active);
    assert_eq!(app_state.main_screen.file_cursor, 1);
    assert_eq!(app_state.main_screen.line_cursor, 1, "Line cursor should move up");

    // Press Arrow Up to deactivate diff cursor and move to header
    app_state = update_state(app_state, Some(Input::KeyUp), 80, 80);
    assert!(!app_state.main_screen.is_diff_cursor_active, "Diff cursor should be inactive after arrow key");
    assert_eq!(app_state.main_screen.file_cursor, 0, "File cursor should move up to header");
    assert_eq!(app_state.main_screen.line_cursor, 0, "Line cursor should reset");
}

#[test]
fn test_main_screen_no_untracked_files() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial commit");
    repo.create_file("a.txt", "hello world");

    let files = git::get_diff(repo.path.clone());
    let app_state = AppState::new(repo.path, files);

    assert!(!app_state
        .unstaged_pane
        .list_items
        .iter()
        .any(|item| matches!(item, UnstagedListItem::UntrackedFilesHeader)));
}
