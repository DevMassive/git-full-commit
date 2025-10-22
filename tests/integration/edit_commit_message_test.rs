use crate::integration::common::{TestRepo, assert_commit_list, get_log, select_commit_in_log};
use git_full_commit::ui::main_screen::ListItem as MainScreenListItem;
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

#[test]
fn test_reorder_edit_horizontal_scroll_ascii() {
    let repo = TestRepo::new();
    repo.commit("base");
    let long_message = "abcdefghijklmnopqrstuvwxyz0123456789";
    repo.commit(long_message);
    repo.commit("top");

    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    state = update_state_with_alt(state, Some(Input::Character('\n')), 80, 24);

    let (offset, extra_space) = match &state.main_screen.list_items[state.main_screen.file_cursor] {
        MainScreenListItem::EditingReorderCommit {
            scroll_offset,
            scroll_extra_space,
            ..
        } => (*scroll_offset, *scroll_extra_space),
        _ => panic!("Expected EditingReorderCommit item"),
    };

    assert!(
        offset > 0,
        "scroll offset should be positive after entering edit mode"
    );
    assert!(
        !extra_space,
        "ASCII-only messages do not need ellipsis padding"
    );

    let expected_offset = {
        use unicode_width::UnicodeWidthStr;
        let prefix_width = " ● ".width();
        let available_width = (24usize).saturating_sub(prefix_width);
        let target_position = available_width.saturating_sub(4);
        long_message.chars().count() + 1usize - target_position
    };
    assert_eq!(offset, expected_offset);

    state = update_state(state, Some(Input::KeyLeft), 80, 24);
    if let MainScreenListItem::EditingReorderCommit { scroll_offset, .. } =
        &state.main_screen.list_items[state.main_screen.file_cursor]
    {
        assert!(
            *scroll_offset <= offset,
            "scroll offset should not increase when moving left"
        );
    }

    for _ in 0..long_message.chars().count() {
        state = update_state(state, Some(Input::KeyLeft), 80, 24);
    }

    match &state.main_screen.list_items[state.main_screen.file_cursor] {
        MainScreenListItem::EditingReorderCommit {
            scroll_offset,
            scroll_extra_space,
            ..
        } => {
            assert_eq!(*scroll_offset, 0);
            assert!(
                !*scroll_extra_space,
                "resetting scroll should clear ellipsis padding"
            );
        }
        _ => panic!("Expected EditingReorderCommit item"),
    }
}

#[test]
fn test_reorder_edit_horizontal_scroll_with_wide_characters() {
    let repo = TestRepo::new();
    repo.commit("base");
    let wide_message = format!("ABCD界{}", "b".repeat(15));
    repo.commit(&wide_message);
    repo.commit("top");

    let mut state = repo.create_initial_state();

    select_commit_in_log(&mut state, 1);
    state = update_state_with_alt(state, Some(Input::Character('\n')), 80, 24);

    let (offset, extra_space) = match &state.main_screen.list_items[state.main_screen.file_cursor] {
        MainScreenListItem::EditingReorderCommit {
            scroll_offset,
            scroll_extra_space,
            ..
        } => (*scroll_offset, *scroll_extra_space),
        _ => panic!("Expected EditingReorderCommit item"),
    };

    assert!(
        offset > 0,
        "scroll offset should advance for wide characters near the cursor"
    );
    assert!(
        extra_space,
        "double-width character boundary should force ellipsis padding"
    );

    state = update_state(state, Some(Input::KeyLeft), 80, 24);
    if let MainScreenListItem::EditingReorderCommit { scroll_offset, .. } =
        &state.main_screen.list_items[state.main_screen.file_cursor]
    {
        assert!(
            *scroll_offset <= offset,
            "scroll offset should not increase when moving left"
        );
    }

    for _ in 0..wide_message.chars().count() {
        state = update_state(state, Some(Input::KeyLeft), 80, 24);
    }

    match &state.main_screen.list_items[state.main_screen.file_cursor] {
        MainScreenListItem::EditingReorderCommit {
            scroll_offset,
            scroll_extra_space,
            ..
        } => {
            assert_eq!(*scroll_offset, 0);
            assert!(
                !*scroll_extra_space,
                "resetting scroll should clear ellipsis padding"
            );
        }
        _ => panic!("Expected EditingReorderCommit item"),
    }
}
