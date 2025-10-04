use crate::ui::main_screen::ListItem;
use crate::ui::update::*;
use pancurses::Input;
use std::process::Command as OsCommand;

use super::other::*;

#[test]
fn test_navigation_in_amend_mode() {
    let repo_path = setup_temp_repo();

    // 1. Create two commits
    std::fs::write(repo_path.join("file1.txt"), "a").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("first commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    std::fs::write(repo_path.join("file2.txt"), "b").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("second commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // 2. Create AppState and find a commit to amend
    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);

    let second_commit_index = state
        .main_screen
        .list_items
        .iter()
        .position(|item| {
            if let crate::ui::main_screen::ListItem::PreviousCommitInfo { message, .. } = item {
                message == "second commit"
            } else {
                false
            }
        })
        .unwrap();

    state.main_screen.file_cursor = second_commit_index;

    // 3. Enter amend mode
    let state_in_amend = update_state(state, Some(Input::Character('\n')), 80, 80);
    assert!(matches!(
        state_in_amend.current_main_item(),
        Some(ListItem::AmendingCommitMessageInput { .. })
    ));
    assert!(state_in_amend.main_screen.amending_commit_hash.is_some());

    // 4. Navigate down
    let state_after_down = update_state(state_in_amend, Some(Input::KeyDown), 80, 80);
    assert!(
        !matches!(
            state_after_down.current_main_item(),
            Some(ListItem::AmendingCommitMessageInput { .. })
        ),
        "Should exit amend mode after navigating down"
    );
    assert!(state_after_down.main_screen.amending_commit_hash.is_none());

    // 5. Navigate up
    let state_after_up = update_state(state_after_down, Some(Input::KeyUp), 80, 80);
    assert!(
        !matches!(
            state_after_up.current_main_item(),
            Some(ListItem::AmendingCommitMessageInput { .. })
        ),
        "Should not be in amend mode after navigating up"
    );

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_amend_commit_message_in_place() {
    let repo_path = setup_temp_repo();
    std::fs::write(repo_path.join("file1.txt"), "a").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("first commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    std::fs::write(repo_path.join("file2.txt"), "b").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("second commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    std::fs::write(repo_path.join("file3.txt"), "c").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("third commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    assert_eq!(state.previous_commits.len(), 3);

    let commit_to_amend_index = state
        .main_screen
        .list_items
        .iter()
        .position(|item| {
            if let crate::ui::main_screen::ListItem::PreviousCommitInfo { message, .. } = item {
                message == "second commit"
            } else {
                false
            }
        })
        .unwrap();

    state.main_screen.file_cursor = commit_to_amend_index;

    let mut state = update_state(state, Some(Input::Character('\n')), 80, 80);
    assert!(matches!(
        state.current_main_item(),
        Some(ListItem::AmendingCommitMessageInput { message, .. }) if message == "second commit"
    ));

    let new_text = " reworded";
    for char_to_add in new_text.chars() {
        state = update_state(state, Some(Input::Character(char_to_add)), 80, 80);
    }

    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    assert!(state.main_screen.amending_commit_hash.is_none());

    let output = OsCommand::new("git")
        .arg("log")
        .arg("--pretty=%s")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    let log_messages = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();
    assert_eq!(log_messages[0], "third commit");
    assert_eq!(log_messages[1], "second commit reworded");
    assert_eq!(log_messages[2], "first commit");

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_amend_commit_with_staged_changes() {
    let repo_path = setup_temp_repo();

    // 1. Create two commits
    std::fs::write(repo_path.join("file1.txt"), "a").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("first commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    std::fs::write(repo_path.join("file2.txt"), "b").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("second commit")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // 2. Stage new changes
    std::fs::write(repo_path.join("file2.txt"), "b-modified").unwrap();
    std::fs::write(repo_path.join("file3.txt"), "c-new").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg("file2.txt")
        .arg("file3.txt")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // 3. Create AppState and find the commit to amend
    let staged_files = crate::git::get_diff(repo_path.clone());
    let mut state = crate::app_state::AppState::new(repo_path.clone(), staged_files);

    let commit_to_amend_index = state
        .main_screen
        .list_items
        .iter()
        .position(|item| {
            if let crate::ui::main_screen::ListItem::PreviousCommitInfo { message, .. } = item {
                message == "second commit"
            } else {
                false
            }
        })
        .unwrap();

    state.main_screen.file_cursor = commit_to_amend_index;

    // 4. Enter amend mode
    let mut state = update_state(state, Some(Input::Character('\n')), 80, 80);
    assert!(matches!(
        state.current_main_item(),
        Some(ListItem::AmendingCommitMessageInput { message, .. }) if message == "second commit"
    ));

    // 5. Change commit message
    if let Some(ListItem::AmendingCommitMessageInput { message, .. }) =
        state.main_screen.list_items.get_mut(commit_to_amend_index)
    {
        *message = "amended second commit".to_string();
    }

    // 6. Execute amend
    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    assert!(state.main_screen.amending_commit_hash.is_none());
    assert!(
        state.error_message.is_none(),
        "Error message should be None. Got: {:?}",
        state.error_message
    );

    // 7. Verify the result
    let log_output = OsCommand::new("git")
        .arg("log")
        .arg("--pretty=%s")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    let log_messages: Vec<String> = String::from_utf8_lossy(&log_output.stdout)
        .lines()
        .map(String::from)
        .collect();

    assert_eq!(log_messages.len(), 2);
    assert_eq!(log_messages[0], "amended second commit");
    assert_eq!(log_messages[1], "first commit");

    let show_output = OsCommand::new("git")
        .arg("show")
        .arg("HEAD")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    let show_str = String::from_utf8_lossy(&show_output.stdout);

    assert!(show_str.contains("file2.txt"));
    assert!(show_str.contains("file3.txt"));
    assert!(show_str.contains("b-modified"));
    assert!(show_str.contains("c-new"));

    std::fs::remove_dir_all(&repo_path).unwrap();
}