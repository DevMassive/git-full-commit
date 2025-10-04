use crate::app_state::Screen;
use crate::ui::update::*;
use pancurses::Input;
use std::process::Command as OsCommand;

use super::other::*;

#[test]
fn test_ignore_file() {
    // Setup a temporary git repository
    let temp_dir = std::env::temp_dir().join("test_repo_for_ignore_v2");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir(&temp_dir).unwrap();
    OsCommand::new("git")
        .arg("init")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to init git repo");
    OsCommand::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to set git user.name");
    OsCommand::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to set git user.email");
    std::fs::write(temp_dir.join("a.txt"), "initial content").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg("a.txt")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to git add");
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial commit")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to git commit");

    // Create a file to be ignored
    let file_to_ignore = "some_file.txt";
    std::fs::write(temp_dir.join(file_to_ignore), "Hello").unwrap();

    // Stage the file
    OsCommand::new("git")
        .arg("add")
        .arg(file_to_ignore)
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to git add");

    // Create initial state
    let files = crate::git::get_diff(temp_dir.clone());
    let mut state = crate::app_state::AppState::new(temp_dir.clone(), files);
    state.main_screen.file_cursor = 1; // Select the file

    // Simulate pressing 'i'
    let mut updated_state = update_state(state, Some(Input::Character('i')), 80, 80);

    // Check if .gitignore is correct
    let gitignore_path = temp_dir.join(".gitignore");
    assert!(gitignore_path.exists(), ".gitignore should be created");
    let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
    assert!(
        gitignore_content.contains(file_to_ignore),
        ".gitignore should contain the ignored file"
    );

    // After ignoring, the file should be gone from the diff,
    // and the .gitignore file should be the only change.
    assert_eq!(
        updated_state.files.len(),
        1,
        "File list should only contain .gitignore"
    );
    assert_eq!(
        updated_state.files[0].file_name, ".gitignore",
        "The remaining file should be .gitignore"
    );

    // Simulate undo
    updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

    // After undo, the original file should be back and .gitignore should be gone
    assert_eq!(
        updated_state.files.len(),
        1,
        "File list should contain the original file again"
    );
    assert_eq!(
        updated_state.files[0].file_name, file_to_ignore,
        "The file should be the one we ignored"
    );

    // Simulate undo
    updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

    // After undo, the original file should be back and .gitignore should be gone
    assert_eq!(
        updated_state.files.len(),
        1,
        "File list should contain the original file again"
    );
    assert_eq!(
        updated_state.files[0].file_name, file_to_ignore,
        "The file should be the one we ignored"
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_discard_hunk() {
    // Setup a temporary git repository
    let temp_dir = std::env::temp_dir().join("test_repo_for_discard_hunk");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir(&temp_dir).unwrap();
    let repo_path = temp_dir;

    OsCommand::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git repo");
    OsCommand::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git user.name");
    OsCommand::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git user.email");

    // Create and commit a file
    let file_path = repo_path.join("test.txt");
    let initial_content = (1..=20)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&file_path, &initial_content).unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg("test.txt")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to git add");
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial commit")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to git commit");

    // Modify the file to create two hunks
    let modified_content = {
        let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
        lines[2] = "modified line 3".to_string(); // Hunk 1
        lines[15] = "modified line 16".to_string(); // Hunk 2
        lines.join("\n")
    };
    std::fs::write(&file_path, &modified_content).unwrap();

    // Stage the changes
    OsCommand::new("git")
        .arg("add")
        .arg("test.txt")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to git add");

    // Create app state
    let files = crate::git::get_diff(repo_path.clone());
    let mut state = crate::app_state::AppState::new(repo_path.clone(), files);
    state.main_screen.file_cursor = 1; // Select the file
    state.main_screen.is_diff_cursor_active = true;
    // Move cursor to the second hunk (around line 16)
    // The diff output will have headers and context lines, so we need to estimate the line number
    let line_in_diff = state.files[0]
        .lines
        .iter()
        .position(|l| l.contains("modified line 16"))
        .unwrap_or(15);
    state.main_screen.line_cursor = line_in_diff;

    // Simulate pressing '!' to discard the hunk
    let updated_state = update_state(state, Some(Input::Character('!')), 80, 80);

    // Check that the hunk is gone from both staged and working directory
    let staged_diff = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get staged diff");
    let staged_diff_str = String::from_utf8_lossy(&staged_diff.stdout);
    assert!(!staged_diff_str.contains("modified line 16"));
    assert!(staged_diff_str.contains("modified line 3")); // First hunk should remain

    let working_diff = OsCommand::new("git")
        .arg("diff")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get working directory diff");
    let working_diff_str = String::from_utf8_lossy(&working_diff.stdout);
    assert!(
        working_diff_str.is_empty(),
        "Working directory should be clean"
    );

    // Simulate undo
    let _updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

    // Check that the hunk is back
    let staged_diff_after_undo = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get staged diff");
    let staged_diff_str_after_undo = String::from_utf8_lossy(&staged_diff_after_undo.stdout);
    assert!(staged_diff_str_after_undo.contains("modified line 16"));
    assert!(staged_diff_str_after_undo.contains("modified line 3"));

    // Cleanup
    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_stage_all_and_undo() {
    // Setup a temporary git repository
    let temp_dir = std::env::temp_dir().join("test_repo_for_stage_all");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir(&temp_dir).unwrap();
    let repo_path = temp_dir;

    OsCommand::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git repo");
    OsCommand::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git user.name");
    OsCommand::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git user.email");

    // Create and commit a file
    let committed_file = repo_path.join("committed.txt");
    std::fs::write(&committed_file, "initial content").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to git add");
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial commit")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to git commit");

    // Modify the committed file
    std::fs::write(&committed_file, "modified content").unwrap();

    // Create a new untracked file
    let untracked_file = repo_path.join("untracked.txt");
    std::fs::write(&untracked_file, "new file").unwrap();

    // Create app state
    let files = crate::git::get_unstaged_diff(&repo_path);
    let state = crate::app_state::AppState::new(repo_path.clone(), files);

    // Simulate pressing 'R'
    let updated_state = update_state(state, Some(Input::Character('R')), 80, 80);

    // Check that both files are staged
    let status_output = OsCommand::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get git status");
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_str.contains("M  committed.txt"));
    assert!(status_str.contains("A  untracked.txt"));

    // Simulate undo
    let _updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

    // Check that the original state is restored
    let status_output_after_undo = OsCommand::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to get git status");
    let status_str_after_undo = String::from_utf8_lossy(&status_output_after_undo.stdout);
    assert!(status_str_after_undo.contains(" M committed.txt"));
    assert!(status_str_after_undo.contains("?? untracked.txt"));

    // Cleanup
    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_unstage_all() {
    let repo_path = setup_temp_repo();
    // Create a committed file
    std::fs::write(repo_path.join("committed.txt"), "a\n").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("i")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create a modified file and a new file, then stage them
    std::fs::write(repo_path.join("committed.txt"), "b\n").unwrap();
    std::fs::write(repo_path.join("new.txt"), "c\n").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    let files = crate::git::get_diff(repo_path.clone());
    let mut state = crate::app_state::AppState::new(repo_path.clone(), files);
    state.main_screen.file_cursor = 0; // Select "Staged changes" header

    // Unstage all
    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains(" M committed.txt"),
        "Should be unstaged modified"
    );
    assert!(status.contains("?? new.txt"), "Should be untracked");

    // Undo
    // Ensure file cursor is on "Staged changes" header
    assert_eq!(state.main_screen.file_cursor, 0);
    let state = update_state(state, Some(Input::Character('<')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains("M  committed.txt"),
        "Should be staged modified"
    );
    assert!(status.contains("A  new.txt"), "Should be staged new");

    // Redo
    let _ = update_state(state, Some(Input::Character('>')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(status.contains(" M committed.txt"));
    assert!(status.contains("?? new.txt"));

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_stage_unstaged() {
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
        .arg("i")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // One modified, one untracked
    std::fs::write(repo_path.join("file1.txt"), "b").unwrap();
    std::fs::write(repo_path.join("untracked.txt"), "c").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 0; // Select "Unstaged changes" header

    // Stage all unstaged
    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(status.contains("M  file1.txt"), "file1 should be staged");
    assert!(
        status.contains("?? untracked.txt"),
        "untracked should remain untracked"
    );

    // Undo
    let state = update_state(state, Some(Input::Character('<')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains(" M file1.txt"),
        "file1 should be unstaged again"
    );
    assert!(
        status.contains("?? untracked.txt"),
        "untracked should still be untracked"
    );

    // Redo
    let _ = update_state(state, Some(Input::Character('>')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(status.contains("M  file1.txt"));
    assert!(status.contains("?? untracked.txt"));

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_stage_untracked() {
    let repo_path = setup_temp_repo();
    std::fs::write(repo_path.join("untracked1.txt"), "a").unwrap();
    std::fs::write(repo_path.join("untracked2.txt"), "b").unwrap();

    // Add a modified file to ensure we only stage untracked
    std::fs::write(repo_path.join("modified.txt"), "c").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg("modified.txt")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("i")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::fs::write(repo_path.join("modified.txt"), "d").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = state.unstaged_screen.unstaged_files.len() + 1; // Select "Untracked files" header

    // Stage all untracked
    let state = update_state(state, Some(Input::Character('\n')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains("A  untracked1.txt"),
        "untracked1 should be staged"
    );
    assert!(
        status.contains("A  untracked2.txt"),
        "untracked2 should be staged"
    );
    assert!(
        status.contains(" M modified.txt"),
        "modified.txt should NOT be staged"
    );

    // Undo
    let state = update_state(state, Some(Input::Character('<')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.contains("?? untracked1.txt"),
        "untracked1 should be untracked again"
    );
    assert!(
        status.contains("?? untracked2.txt"),
        "untracked2 should be untracked again"
    );
    assert!(
        status.contains(" M modified.txt"),
        "modified.txt should be untouched"
    );

    // Redo
    let _ = update_state(state, Some(Input::Character('>')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(status.contains("A  untracked1.txt"));
    assert!(status.contains("A  untracked2.txt"));
    assert!(status.contains(" M modified.txt"));

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_discard_unstaged_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("test.txt");
    std::fs::write(&file_path, "line1\nline2\n").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::fs::write(&file_path, "line1\nMODIFIED\n").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 1;

    let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
    let status = get_git_status(&repo_path);
    assert!(
        status.is_empty(),
        "Git status should be clean after discard"
    );
    assert!(
        state_after_discard
            .unstaged_screen
            .unstaged_files
            .is_empty()
    );

    let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
    let status_after_undo = get_git_status(&repo_path);
    assert!(
        status_after_undo.contains(" M test.txt"),
        "File should be modified again after undo"
    );
    assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);
    assert_eq!(
        state_after_undo.unstaged_screen.unstaged_files[0].file_name,
        "test.txt"
    );

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_discard_untracked_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("untracked.txt");
    std::fs::write(&file_path, "hello").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2; // Header, Header, File

    let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
    assert!(!file_path.exists(), "File should be deleted");
    assert!(
        state_after_discard
            .unstaged_screen
            .untracked_files
            .is_empty()
    );

    let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
    assert!(file_path.exists(), "File should be restored after undo");
    assert_eq!(state_after_undo.unstaged_screen.untracked_files.len(), 1);
    assert_eq!(
        state_after_undo.unstaged_screen.untracked_files[0],
        "untracked.txt"
    );

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_discard_untracked_binary_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("binary.bin");
    std::fs::write(&file_path, b"hello\0world").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2;

    let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
    assert!(file_path.exists(), "Binary file should not be deleted");
    assert_eq!(state_after_discard.unstaged_screen.untracked_files.len(), 1);

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_ignore_unstaged_file() {
    let repo_path = setup_temp_repo();
    let file_path = repo_path.join("test.txt");
    std::fs::write(&file_path, "initial").unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::fs::write(&file_path, "modified").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 1;

    let state_after_ignore = update_state(state, Some(Input::Character('i')), 80, 80);
    let gitignore_content =
        std::fs::read_to_string(repo_path.join(".gitignore")).unwrap_or_default();
    assert!(gitignore_content.contains("test.txt"));
    let status = get_git_status(&repo_path);
    assert!(status.contains("A  .gitignore"));
    assert!(state_after_ignore.unstaged_screen.unstaged_files.is_empty());

    let state_after_undo = update_state(state_after_ignore, Some(Input::Character('<')), 80, 80);
    let status_after_undo = get_git_status(&repo_path);
    assert!(!repo_path.join(".gitignore").exists());
    assert!(status_after_undo.contains(" M test.txt"));
    assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);

    std::fs::remove_dir_all(&repo_path).unwrap();
}

#[test]
fn test_ignore_untracked_file() {
    let repo_path = setup_temp_repo();
    std::fs::write(repo_path.join("untracked.txt"), "hello").unwrap();

    let mut state = crate::app_state::AppState::new(repo_path.clone(), vec![]);
    state.refresh_diff();
    state.screen = Screen::Unstaged;
    state.unstaged_screen.unstaged_cursor = 2;

    let state_after_ignore = update_state(state, Some(Input::Character('i')), 80, 80);
    let gitignore_content =
        std::fs::read_to_string(repo_path.join(".gitignore")).unwrap_or_default();
    assert!(gitignore_content.contains("untracked.txt"));
    let status = get_git_status(&repo_path);
    assert!(status.contains("A  .gitignore"));
    assert!(!status.contains("untracked.txt"));
    assert!(
        state_after_ignore
            .unstaged_screen
            .untracked_files
            .is_empty()
    );

    let state_after_undo = update_state(state_after_ignore, Some(Input::Character('<')), 80, 80);
    let status_after_undo = get_git_status(&repo_path);
    assert!(!repo_path.join(".gitignore").exists());
    assert!(status_after_undo.contains("?? untracked.txt"));
    assert_eq!(state_after_undo.unstaged_screen.untracked_files.len(), 1);

    std::fs::remove_dir_all(&repo_path).unwrap();
}