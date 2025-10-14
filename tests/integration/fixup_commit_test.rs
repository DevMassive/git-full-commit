use crate::integration::common::{select_commit_in_log, TestRepo};
use git_full_commit::ui::main_screen::ListItem;
use git_full_commit::ui::update::{update_state, update_state_with_alt};
use pancurses::Input;

#[test]
fn test_fixup_commit_in_reorder_mode() {
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");
    let mut state = repo.create_initial_state();

    // The commit log is ["commit 2", "commit 1", "commit 0"]
    // Select "commit 1" (index 1) to enter reorder mode
    select_commit_in_log(&mut state, 1);
    state = update_state_with_alt(state, Some(Input::KeyUp), 1024, 1024); // Enter reorder mode

    // After entering reorder mode, "commit 1" and "commit 2" are swapped,
    // and the cursor is on "commit 1".

    // Press 'f' to mark the selected commit ("commit 1") as a fixup
    state = update_state(state, Some(Input::Character('f')), 1024, 1024);

    // Assert that the message for "commit 1" is now "fixup!"
    if let ListItem::PreviousCommitInfo { is_fixup, .. } =
        &state.main_screen.list_items[state.main_screen.file_cursor]
    {
        assert!(*is_fixup);
    } else {
        panic!("Expected a PreviousCommitInfo item");
    }

    // Confirm the reorder
    state = update_state(state, Some(Input::Character('\n')), 1024, 1024);

    // After reordering, "commit 1" should be squashed into "commit 0".
    // The final commit log should contain "commit 2" and "commit 0".
    assert_eq!(state.previous_commits.len(), 2);
    assert_eq!(state.previous_commits[0].message, "commit 2");
    assert_eq!(state.previous_commits[1].message, "commit 0");
}