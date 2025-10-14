use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::main_screen::ListItem as MainScreenListItem;
use git_full_commit::ui::update::{update_state, update_state_with_alt};
use pancurses::Input;

#[test]
fn test_commit_message_input_and_commit() {
    // Setup repo with a staged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Navigate to commit message input
    // list is [Header, File, Input], so cursor should be at 2
    app_state.main_screen.file_cursor = 2;
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80); // Press enter to activate
    assert!(app_state.is_in_input_mode());

    // Type a commit message
    let commit_message = "Test commit";
    for ch in commit_message.chars() {
        app_state = update_state(app_state, Some(Input::Character(ch)), 80, 80);
    }
    assert_eq!(app_state.main_screen.commit_message, commit_message);

    // Press enter to commit
    update_state(app_state, Some(Input::Character('\n')), 80, 80);

    // Check git log
    let log = repo.get_log(1);
    assert!(log.contains(commit_message));
}

#[test]
fn test_amend_commit_reword_only() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    let initial_message = "initial commit";
    repo.commit(initial_message);

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Navigate to the commit
    // list is [Staged H, Input, Commit], so cursor is at 2
    app_state.main_screen.file_cursor = 2;

    // Press enter to start amending
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);
    assert!(matches!(
        app_state.main_screen.list_items[2],
        MainScreenListItem::AmendingCommitMessageInput { .. }
    ));

    // Clear the old message
    for _ in 0..initial_message.len() {
        app_state = update_state(app_state, Some(Input::KeyBackspace), 80, 80);
    }

    // Change the commit message
    let new_message = "new message";
    for ch in new_message.chars() {
        app_state = update_state(app_state, Some(Input::Character(ch)), 80, 80);
    }

    // Press enter to finalize
    update_state(app_state, Some(Input::Character('\n')), 80, 80);

    let log = repo.get_log(1);
    assert!(log.contains(new_message));
    assert!(!log.contains(initial_message));
}

#[test]
fn test_amend_commit_with_staged_changes() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial commit");

    repo.create_file("b.txt", "staged file");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Navigate to the commit
    // list is [Staged H, b.txt, Input, Commit], so cursor is at 3
    app_state.main_screen.file_cursor = 3;

    // Press enter to start amending
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);

    // Clear the old message
    for _ in 0.."initial commit".len() {
        app_state = update_state(app_state, Some(Input::KeyBackspace), 80, 80);
    }

    // Change the commit message
    let new_message = "new message";
    for ch in new_message.chars() {
        app_state = update_state(app_state, Some(Input::Character(ch)), 80, 80);
    }

    // Press enter to finalize
    update_state(app_state, Some(Input::Character('\n')), 80, 80);

    let log = repo.get_log(1);
    assert!(log.contains(new_message));

    // Check that the staged file is in the commit
    let diff = repo.get_commit_diff("HEAD");
    assert!(diff.contains("b.txt"));
}

#[test]
fn test_amend_is_disabled_for_remote_commit() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial commit");
    repo.push();

    repo.create_file("b.txt", "local");
    repo.add_all();
    repo.commit("local commit");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // list is [Header, Input, local commit, remote commit]
    let remote_commit_index = 3;
    let local_commit_index = 2;

    // Verify the is_on_remote flags
    assert_eq!(app_state.previous_commits.len(), 2);
    let local_commit = &app_state.previous_commits[0];
    let remote_commit = &app_state.previous_commits[1];
    assert_eq!(local_commit.message, "local commit");
    assert!(!local_commit.is_on_remote);
    assert_eq!(remote_commit.message, "initial commit");
    assert!(remote_commit.is_on_remote, "Commit should be on remote");

    // Navigate to the remote commit
    app_state.main_screen.file_cursor = remote_commit_index;

    // Press enter, should not start amending
    let state_before = app_state.main_screen.file_cursor;
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);
    assert_eq!(app_state.main_screen.file_cursor, state_before);
    assert!(matches!(
        app_state.main_screen.list_items[remote_commit_index],
        MainScreenListItem::PreviousCommitInfo { .. }
    ));

    // Navigate to the local commit
    app_state.main_screen.file_cursor = local_commit_index;

    // Press enter, should start amending
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);
    assert!(matches!(
        app_state.main_screen.list_items[local_commit_index],
        MainScreenListItem::AmendingCommitMessageInput { .. }
    ));
}

#[test]
fn test_commit_message_word_movement() {
    // Setup repo with a staged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Navigate to commit message input
    app_state.main_screen.file_cursor = 2;
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80); // Press enter to activate

    // Type a commit message
    let commit_message = "word1 word2 word3";
    for ch in commit_message.chars() {
        app_state = update_state(app_state, Some(Input::Character(ch)), 80, 80);
    }
    assert_eq!(app_state.main_screen.commit_message, commit_message);
    assert_eq!(app_state.main_screen.commit_cursor, commit_message.len());

    // Test meta+left
    app_state = update_state_with_alt(app_state, Some(Input::KeyLeft), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 word2 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::KeyLeft), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::KeyLeft), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, 0);

    // Test meta+right
    app_state = update_state_with_alt(app_state, Some(Input::KeyRight), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::KeyRight), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 word2 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::KeyRight), 80, 80);
    assert_eq!(
        app_state.main_screen.commit_cursor,
        "word1 word2 word3".len()
    );

    // Test meta+backspace
    app_state = update_state_with_alt(app_state, Some(Input::KeyBackspace), 80, 80);
    assert_eq!(app_state.main_screen.commit_message, "word1 word2 ");
    assert_eq!(app_state.main_screen.commit_cursor, "word1 word2 ".len());

    app_state = update_state_with_alt(app_state, Some(Input::KeyBackspace), 80, 80);
    assert_eq!(app_state.main_screen.commit_message, "word1 ");
    assert_eq!(app_state.main_screen.commit_cursor, "word1 ".len());

    app_state = update_state_with_alt(app_state, Some(Input::KeyBackspace), 80, 80);
    assert_eq!(app_state.main_screen.commit_message, "");
    assert_eq!(app_state.main_screen.commit_cursor, 0);
}

#[test]
fn test_commit_message_word_movement_bf() {
    // Setup repo with a staged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Navigate to commit message input
    app_state.main_screen.file_cursor = 2;
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80); // Press enter to activate

    // Type a commit message
    let commit_message = "word1 word2 word3";
    for ch in commit_message.chars() {
        app_state = update_state(app_state, Some(Input::Character(ch)), 80, 80);
    }
    assert_eq!(app_state.main_screen.commit_message, commit_message);
    assert_eq!(app_state.main_screen.commit_cursor, commit_message.len());

    // Test meta+b
    app_state = update_state_with_alt(app_state, Some(Input::Character('b')), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 word2 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::Character('b')), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::Character('b')), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, 0);

    // Test meta+f
    app_state = update_state_with_alt(app_state, Some(Input::Character('f')), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::Character('f')), 80, 80);
    assert_eq!(app_state.main_screen.commit_cursor, "word1 word2 ".len());
    app_state = update_state_with_alt(app_state, Some(Input::Character('f')), 80, 80);
    assert_eq!(
        app_state.main_screen.commit_cursor,
        "word1 word2 word3".len()
    );
}
