use crate::integration::common::{TestRepo, assert_commit_list, get_log, select_commit_in_log};
use git_full_commit::ui::update::{update_state, update_state_with_alt};
use pancurses::Input;

#[test]
fn test_edit_commit_message() {
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");
    let mut state = repo.create_initial_state();

    // Select "commit 1"
    select_commit_in_log(&mut state, 1);

    // Press Option + Enter to start editing.
    state = update_state_with_alt(state, Some(Input::Character('\n')), 1024, 1024); // Enter

    // We are now in reorder mode and editing the commit message.
    // The commit list should be the same, but the UI is in a different mode.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Type a new message. Let's change "commit 1" to "a new message".
    // First, backspace to delete "commit 1" (8 chars).
    for _ in 0..8 {
        state = update_state(state, Some(Input::KeyBackspace), 1024, 1024);
    }
    // Then, type the new message.
    let new_message = "a new message";
    for ch in new_message.chars() {
        state = update_state(state, Some(Input::Character(ch)), 1024, 1024);
    }

    // Press Enter to confirm the message edit.
    state = update_state(state, Some(Input::Character('\n')), 1024, 1024);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", new_message, "commit 0"],
    );

    // Press Enter again to confirm the reorder
    let _ = update_state(state, Some(Input::Character('\n')), 1024, 1024);

    // Now the reorder is complete, and the git history should be updated.
    let log = get_log(&repo.path);
    let messages: Vec<String> = log.iter().map(|c| c.message.clone()).collect();
    assert_eq!(messages, vec!["commit 2", new_message, "commit 0"]);
}

#[test]
fn test_cancel_edit_commit_message() {
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");
    let mut state = repo.create_initial_state();

    // Select "commit 1"
    select_commit_in_log(&mut state, 1);

    // Press Option + Enter to start editing.
    state = update_state_with_alt(state, Some(Input::Character('\n')), 1024, 1024); // Enter

    // Type something
    state = update_state(state, Some(Input::Character('a')), 1024, 1024);

    // Press Ctrl+C to cancel the edit
    state = update_state(state, Some(Input::Character('\u{3}')), 1024, 1024);

    // We should still be in reorder mode, but the message should be restored.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Press Esc to cancel the reorder.
    let _ = update_state(state, Some(Input::Character('\u{1b}')), 1024, 1024); // ESC

    // The log should be unchanged.
    let log = get_log(&repo.path);
    let messages: Vec<String> = log.iter().map(|c| c.message.clone()).collect();
    assert_eq!(messages, vec!["commit 2", "commit 1", "commit 0"]);
}
