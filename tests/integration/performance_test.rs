use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use std::thread;
use std::time::Duration;

#[test]
fn test_debounce_diff_update() {
    let repo = TestRepo::new();
    repo.create_file("test.txt", "content");
    repo.add_all();
    repo.commit("Initial commit");

    let files = git::get_diff(repo.path.clone());
    let mut state = AppState::new(repo.path.clone(), files);

    // Select the commit (it should be after file list and input)
    // Files list might be empty if no changes, but we just committed.
    // If working dir is clean, files is empty.
    // List items: [StagedHeader, CommitInput, Commit1]
    // Index 0: StagedHeader
    // Index 1: CommitInput
    // Index 2: Commit1
    state.main_screen.file_cursor = 2;

    // Initially, last_interaction_time is None
    assert!(state.last_interaction_time.is_none());

    // Trigger debounce
    state.debounce_diff_update();
    assert!(state.last_interaction_time.is_some());

    // Check immediately - should be false (too soon)
    assert!(!state.check_diff_update());
    // last_interaction_time should still be some
    assert!(state.last_interaction_time.is_some());

    // Sleep for a bit, but less than 200ms
    thread::sleep(Duration::from_millis(50));
    assert!(!state.check_diff_update());

    // Sleep enough to pass 200ms total
    thread::sleep(Duration::from_millis(200));

    // Now it should trigger the request and reset timer
    // check_diff_update returns false now because it doesn't trigger render directly
    assert!(!state.check_diff_update());
    assert!(state.last_interaction_time.is_none());

    // We need to wait for the background thread to finish
    let mut updated = false;
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(50));
        if state.poll_background() {
            updated = true;
            break;
        }
    }
    assert!(updated, "Background worker did not update state in time");
}
