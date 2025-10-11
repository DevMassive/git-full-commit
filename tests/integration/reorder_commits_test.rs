use crate::integration::common::{
    assert_commit_list,
    generate_test_repo_and_pancurses,
    select_commit_in_log,
};
use git_full_commit::ui::main_screen::ListItem;
use pancurses::Input;

#[test]
fn test_reorder_commits() {
    let (repo, mut pancurses) = generate_test_repo_and_pancurses(1024, 1024);
    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    pancurses.send_input(Input::Character('\u{1b}')); // ESC
    state = repo.update_state(state, &mut pancurses);
    pancurses.send_input(Input::KeyUp);
    state = repo.update_state(state, &mut pancurses);

    // After entering reorder mode and moving up, "commit 1" and "commit 2" should swap places.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 1", "commit 2", "commit 0"],
    );

    // Move down
    pancurses.send_input(Input::KeyDown);
    state = repo.update_state(state, &mut pancurses);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Confirm reorder
    pancurses.send_input(Input::Character('\n'));
    let state = repo.update_state(state, &mut pancurses);
    assert_eq!(
        state
            .previous_commits
            .iter()
            .map(|c| c.message.clone())
            .collect::<Vec<_>>(),
        vec!["commit 2", "commit 1", "commit 0"]
    );
}

#[test]
fn test_discard_commit_in_reorder_mode() {
    let (repo, mut pancurses) = generate_test_repo_and_pancurses(1024, 1024);
    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    pancurses.send_input(Input::Character('\u{1b}')); // ESC
    state = repo.update_state(state, &mut pancurses);
    pancurses.send_input(Input::KeyUp);
    state = repo.update_state(state, &mut pancurses);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 1", "commit 2", "commit 0"],
    );

    // Discard "commit 1"
    pancurses.send_input(Input::Character('!'));
    state = repo.update_state(state, &mut pancurses);
    assert_commit_list(&state.main_screen.list_items, &["commit 2", "commit 0"]);

    // Confirm reorder
    pancurses.send_input(Input::Character('\n'));
    let state = repo.update_state(state, &mut pancurses);

    assert_eq!(
        state
            .previous_commits
            .iter()
            .map(|c| c.message.clone())
            .collect::<Vec<_>>(),
        vec!["commit 2", "commit 0"]
    );
}

#[test]
fn test_undo_discard_in_reorder_mode() {
    let (repo, mut pancurses) = generate_test_repo_and_pancurses(1024, 1024);
    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    pancurses.send_input(Input::Character('\u{1b}')); // ESC
    state = repo.update_state(state, &mut pancurses);
    pancurses.send_input(Input::KeyUp);
    state = repo.update_state(state, &mut pancurses);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 1", "commit 2", "commit 0"],
    );

    // Discard "commit 1"
    pancurses.send_input(Input::Character('!'));
    state = repo.update_state(state, &mut pancurses);
    assert_commit_list(&state.main_screen.list_items, &["commit 2", "commit 0"]);

    // Undo discard
    pancurses.send_input(Input::Character('<'));
    state = repo.update_state(state, &mut pancurses);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Confirm reorder
    pancurses.send_input(Input::Character('\n'));
    let state = repo.update_state(state, &mut pancurses);
    assert_eq!(
        state
            .previous_commits
            .iter()
            .map(|c| c.message.clone())
            .collect::<Vec<_>>(),
        vec!["commit 2", "commit 1", "commit 0"]
    );
}
