use crate::integration::common::{
    assert_commit_list, generate_test_repo_and_pancurses, get_log, select_commit_in_log,
};
use pancurses::Input;

#[test]
fn test_edit_commit_message() {
    let (repo, mut pancurses) = generate_test_repo_and_pancurses(1024, 1024);
    let mut state = repo.create_initial_state();

    // Select "commit 1"
    select_commit_in_log(&mut state, 1);

    // Press Option + Enter to start editing.
    // pancurses represents this as ESC followed by the character.
    pancurses.send_input(Input::Character('\u{1b}')); // ESC
    state = repo.update_state(state, &mut pancurses);
    pancurses.send_input(Input::Character('\n')); // Enter
    state = repo.update_state(state, &mut pancurses);

    // We are now in reorder mode and editing the commit message.
    // The commit list should be the same, but the UI is in a different mode.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Type a new message. Let's change "commit 1" to "a new message".
    // First, backspace to delete "commit 1" (8 chars).
    for _ in 0..8 {
        pancurses.send_input(Input::KeyBackspace);
        state = repo.update_state(state, &mut pancurses);
    }
    // Then, type the new message.
    let new_message = "a new message";
    for ch in new_message.chars() {
        pancurses.send_input(Input::Character(ch));
        state = repo.update_state(state, &mut pancurses);
    }

    // Press Enter to confirm the message edit.
    pancurses.send_input(Input::Character('\n'));
    state = repo.update_state(state, &mut pancurses);

    // We are still in reorder mode. The list should now show the new message.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", new_message, "commit 0"],
    );

    // Press Enter again to confirm the reorder.
    pancurses.send_input(Input::Character('\n'));
    let state = repo.update_state(state, &mut pancurses);

    // Now the reorder is complete, and the git history should be updated.
    let log = get_log(&repo.path);
    let messages: Vec<String> = log.iter().map(|c| c.message.clone()).collect();
    assert_eq!(messages, vec!["commit 2", new_message, "commit 0"]);
}

#[test]
fn test_cancel_edit_commit_message() {
    let (repo, mut pancurses) = generate_test_repo_and_pancurses(1024, 1024);
    let mut state = repo.create_initial_state();

    // Select "commit 1"
    select_commit_in_log(&mut state, 1);

    // Press Option + Enter to start editing.
    pancurses.send_input(Input::Character('\u{1b}')); // ESC
    state = repo.update_state(state, &mut pancurses);
    pancurses.send_input(Input::Character('\n')); // Enter
    state = repo.update_state(state, &mut pancurses);

    // Type something
    pancurses.send_input(Input::Character('a'));
    state = repo.update_state(state, &mut pancurses);

    // Press Ctrl+C to cancel the edit
    pancurses.send_input(Input::Character('\u{3}'));
    state = repo.update_state(state, &mut pancurses);

    // We should still be in reorder mode, but the message should be restored.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Press Esc to cancel the reorder.
    pancurses.send_input(Input::Character('\u{1b}')); // ESC
    let state = repo.update_state(state, &mut pancurses);

    // The log should be unchanged.
    let log = get_log(&repo.path);
    let messages: Vec<String> = log.iter().map(|c| c.message.clone()).collect();
    assert_eq!(messages, vec!["commit 2", "commit 1", "commit 0"]);
}
