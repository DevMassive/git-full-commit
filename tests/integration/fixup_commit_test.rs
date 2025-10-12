use crate::integration::common::{
    assert_commit_list, generate_test_repo_and_pancurses, select_commit_in_log,
};
use git_full_commit::ui::main_screen::ListItem;
use pancurses::Input;

#[test]
fn test_fixup_commit_in_reorder_mode() {
    let (repo, mut pancurses) = generate_test_repo_and_pancurses(1024, 1024);
    let mut state = repo.create_initial_state();

    // The commit log is ["commit 2", "commit 1", "commit 0"]
    // Select "commit 1" (index 1) to enter reorder mode
    select_commit_in_log(&mut state, 1);
    pancurses.send_input(Input::Character('\u{1b}')); // ESC (Meta key)
    state = repo.update_state(state, &mut pancurses);
    pancurses.send_input(Input::KeyUp); // Enter reorder mode
    state = repo.update_state(state, &mut pancurses);

    // After entering reorder mode, the selection is on "commit 2" (index 0).
    // Let's move down to select "commit 1" (index 1)
    pancurses.send_input(Input::KeyDown);
    state = repo.update_state(state, &mut pancurses);

    // Press 'f' to mark "commit 1" as a fixup
    pancurses.send_input(Input::Character('f'));
    state = repo.update_state(state, &mut pancurses);

    // Assert that the message for "commit 1" is now "fixup!"
    if let ListItem::PreviousCommitInfo { is_fixup, .. } = &state.main_screen.list_items[1] {
        assert!(*is_fixup);
    } else {
        panic!("Expected a PreviousCommitInfo item");
    }

    // Confirm the reorder
    pancurses.send_input(Input::Character('\n'));
    let state = repo.update_state(state, &mut pancurses);

    // After reordering, "commit 1" should be squashed into "commit 0".
    // The final commit log should contain "commit 2" and "commit 0".
    assert_eq!(state.previous_commits.len(), 2);
    assert_eq!(state.previous_commits[0].message, "commit 2");
    assert_eq!(state.previous_commits[1].message, "commit 0");
}
