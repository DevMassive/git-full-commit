use crate::git_test::common::TestRepo;
use git_full_commit::app_state::{AppState, FocusedPane};
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use git_full_commit::ui::main_screen::UnstagedListItem;
use pancurses::Input;

#[test]
fn test_tab_toggles_screen() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.focused_pane, FocusedPane::Main);

    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.focused_pane, FocusedPane::Unstaged);

    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.focused_pane, FocusedPane::Main);
}

#[test]
fn test_screen_switching_is_blocked_in_commit_mode() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Enter commit mode
    app_state.main_screen.file_cursor = 2;
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);
    assert!(app_state.is_in_input_mode());

    // Try to switch screen
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.focused_pane, FocusedPane::Main, "Should not switch screen in commit mode");
}

#[test]
fn test_cursor_restoration_on_switch() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial");

    // Staged change
    repo.create_file("a.txt", "world"); 
    repo.add_all();

    // Unstaged change
    repo.create_file("a.txt", "unstaged");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Select a.txt on main screen
    app_state.main_screen.file_cursor = 1;

    // Switch to unstaged screen
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    
    // The cursor should be on a.txt on the unstaged screen
    let selected_unstaged_file = &app_state.unstaged_pane.list_items[app_state.unstaged_pane.cursor];
    match selected_unstaged_file {
        UnstagedListItem::File(f) => {
            assert_eq!(f.file_name, "a.txt");
        }
        _ => panic!("Expected a file to be selected"),
    }
}
