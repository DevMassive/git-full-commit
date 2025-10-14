use crate::{
    git_test::common::TestRepo,
    integration::common::{assert_commit_list, select_commit_in_log},
};
use git_full_commit::ui::update::{update_state, update_state_with_alt};
use pancurses::Input;

#[test]
fn test_reorder_commits() {
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");

    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    state = update_state_with_alt(state, Some(Input::KeyUp), 80, 80);

    // After entering reorder mode and moving up, "commit 1" and "commit 2" should swap places.
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 1", "commit 2", "commit 0"],
    );

    // Move down
    state = update_state_with_alt(state, Some(Input::KeyDown), 80, 80);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Confirm reorder
    state = update_state(state, Some(Input::Character('\n')), 80, 80);
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
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");

    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    state = update_state_with_alt(state, Some(Input::KeyUp), 80, 80);

    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 1", "commit 2", "commit 0"],
    );

    // Discard "commit 1"
    state = update_state(state, Some(Input::Character('!')), 80, 80);
    assert_commit_list(&state.main_screen.list_items, &["commit 2", "commit 0"]);

    // Confirm reorder
    state = update_state(state, Some(Input::Character('\n')), 80, 80);

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
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");

    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    state = update_state_with_alt(state, Some(Input::KeyUp), 80, 80);

    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 1", "commit 2", "commit 0"],
    );

    // Discard "commit 1"
    state = update_state(state, Some(Input::Character('!')), 80, 80);
    assert_commit_list(&state.main_screen.list_items, &["commit 2", "commit 0"]);

    // Undo discard
    state = update_state(state, Some(Input::Character('<')), 80, 80);
    assert_commit_list(
        &state.main_screen.list_items,
        &["commit 2", "commit 1", "commit 0"],
    );

    // Confirm reorder
    state = update_state(state, Some(Input::Character('\n')), 80, 80);
    assert_eq!(
        state
            .previous_commits
            .iter()
            .map(|c| c.message.clone())
            .collect::<Vec<_>>(),
        vec!["commit 2", "commit 1", "commit 0"]
    );
}
