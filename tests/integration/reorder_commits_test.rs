use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::main_screen;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

#[test]
fn test_reorder_commits_integration() {
    let repo = TestRepo::new();
    repo.commit("first");
    repo.commit("second");
    repo.commit("third");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // In the commit log, commits are listed in reverse chronological order.
    // With no staged files, the cursor starts at index 0.
    // The list items will be:
    // 0: Staged Changes header
    // 1: Commit Message Input
    // 2: "third"
    // 3: "second"
    // 4: "first"

    // Navigate to the "second" commit (index 3)
    app_state = update_state(app_state, Some(Input::KeyDown), 80, 80); // -> 1
    app_state = update_state(app_state, Some(Input::KeyDown), 80, 80); // -> 2
    app_state = update_state(app_state, Some(Input::KeyDown), 80, 80); // -> 3, selects "second"

    // Sanity check: ensure "second" is selected
    match &app_state.main_screen.list_items[app_state.main_screen.file_cursor] {
        main_screen::ListItem::PreviousCommitInfo { message, .. } => {
            assert!(message.starts_with("second"))
        }
        _ => panic!("Expected a commit to be selected"),
    }

    // Enter reorder mode and move "second" up, swapping it with "third"
    app_state = update_state(app_state, Some(Input::Character('\u{1b}')), 80, 80); // Esc
    app_state = update_state(app_state, Some(Input::KeyUp), 80, 80);

    // After the swap, the cursor should now be at index 2, selecting "second"
    assert_eq!(app_state.main_screen.file_cursor, 2);
    match &app_state.main_screen.list_items[app_state.main_screen.file_cursor] {
        main_screen::ListItem::PreviousCommitInfo { message, .. } => {
            assert!(message.starts_with("second"))
        }
        _ => panic!("Expected a commit to be selected"),
    }
    // And "third" should now be at index 3
    match &app_state.main_screen.list_items[3] {
        main_screen::ListItem::PreviousCommitInfo { message, .. } => {
            assert!(message.starts_with("third"))
        }
        _ => panic!("Expected a commit to be at this position"),
    }

    // Confirm the reorder
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);

    // Check the final git log
    let log = git::get_local_commits(&repo.path).unwrap();
    let commits: Vec<&String> = log.iter().map(|c| &c.message).collect();

    // The new order should be "second", "third", "first"
    let expected_commits: Vec<&str> = vec!["second", "third", "first"];
    assert_eq!(commits, expected_commits);
}
