use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

#[test]
fn test_simple_undo_redo() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.files.len(), 1);

    // Unstage the file
    app_state.main_screen.file_cursor = 1;
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);
    assert_eq!(app_state.files.len(), 0);

    // Undo
    app_state = update_state(app_state, Some(Input::Character('<')), 80, 80);
    assert_eq!(app_state.files.len(), 1);

    // Redo
    app_state = update_state(app_state, Some(Input::Character('>')), 80, 80);
    assert_eq!(app_state.files.len(), 0);
}

#[test]
fn test_undo_redo_history_clears_on_commit() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Unstage
    app_state.main_screen.file_cursor = 1;
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);
    assert_eq!(app_state.files.len(), 0);

    // Undo
    app_state = update_state(app_state, Some(Input::Character('<')), 80, 80);
    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.command_history.redo_stack.len(), 1);

    // Commit
    app_state.main_screen.file_cursor = 2; // Commit input
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80); // Activate
    for ch in "commit".chars() {
        app_state = update_state(app_state, Some(Input::Character(ch)), 80, 80);
    }
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80); // Finalize

    // Check that redo stack is empty
    assert_eq!(app_state.command_history.redo_stack.len(), 0);
    let undo_stack_len_before = app_state.command_history.undo_stack.len();

    // Pressing > should do nothing
    app_state = update_state(app_state, Some(Input::Character('>')), 80, 80);
    assert_eq!(app_state.command_history.undo_stack.len(), undo_stack_len_before);
}

#[test]
fn test_undo_restores_cursor_position() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Move cursor to the file
    app_state.main_screen.file_cursor = 1;
    let cursor_before = app_state.main_screen.file_cursor;

    // Perform an action that moves the cursor (unstage last file)
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);
    // Cursor moves to commit input
    assert_ne!(app_state.main_screen.file_cursor, cursor_before);

    // Undo
    app_state = update_state(app_state, Some(Input::Character('<')), 80, 80);

    // Cursor should be restored
    assert_eq!(app_state.main_screen.file_cursor, cursor_before);
}