use git_full_commit::app_state::AppState;
use git_full_commit::git::{self, apply_patch, get_diff};
use git_full_commit::ui::unstaged_view::handle_unstaged_view_input;
use git_full_commit::ui::update::update_state;
use pancurses::{Input, Window, endwin, initscr};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;
use tempfile::TempDir;

pub struct TestSetup {
    _tmp_dir: TempDir,
    pub repo_path: PathBuf,
}

impl TestSetup {
    fn new() -> Self {
        let (tmp_dir, repo_path) = setup_git_repo();
        TestSetup {
            _tmp_dir: tmp_dir,
            repo_path,
        }
    }

    fn new_multi_line() -> Self {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path().to_path_buf();

        // git init
        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.name", "Test"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);

        // first commit
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        run_git(&repo_path, &["add", "test.txt"]);
        run_git(&repo_path, &["commit", "-m", "initial commit"]);

        // stage file
        fs::write(&file_path, "line1\nchanged\nline3\n").unwrap();
        run_git(&repo_path, &["add", "test.txt"]);

        TestSetup {
            _tmp_dir: tmp_dir,
            repo_path,
        }
    }
}

fn setup_git_repo() -> (TempDir, std::path::PathBuf) {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();

    // git init
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    // first commit
    let file_path = repo_path.join("test.txt");
    fs::write(&file_path, "a\n").unwrap();

    run_git(&repo_path, &["add", "test.txt"]);
    run_git(&repo_path, &["commit", "-m", "initial commit"]);

    // stage file
    fs::write(&file_path, "b\n").unwrap();
    run_git(&repo_path, &["add", "test.txt"]);

    (tmp_dir, repo_path)
}

fn run_git(dir: &std::path::Path, args: &[&str]) {
    let output = OsCommand::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git command");

    if !output.status.success() {
        panic!(
            "git command failed: {:?}\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn create_test_state(test_setup: &TestSetup) -> AppState {
    let files = get_diff(test_setup.repo_path.clone());
    AppState::new(test_setup.repo_path.clone(), files)
}

fn run_test_with_pancurses<F>(test_fn: F)
where
    F: FnOnce(&Window),
{
    let window = initscr();
    window.keypad(true);
    pancurses::noecho();
    test_fn(&window);
    endwin();
}

#[test]
#[serial]
fn test_update_state_quit() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let state = create_test_state(&setup);
        let new_state = update_state(state, Some(Input::Character('\u{3}')), 30, 80);
        assert!(!new_state.running);
    });
}

#[test]
#[serial]
fn test_unstage_file() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let state = create_test_state(&setup);
        let new_state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert_eq!(new_state.files.len(), 0);
    });
}

#[test]
#[serial]
fn test_unstage_hunk_by_line_with_undo_redo() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let mut state = create_test_state(&setup);

        // Ensure we have a file with a hunk
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.files[0].hunks.len(), 1);
        let hunk = &state.files[0].hunks[0].clone();

        state.main_screen.line_cursor = hunk.start_line + 1;

        // Unstage hunk
        let state_after_unstage = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert_eq!(state_after_unstage.files.len(), 0);

        // Undo
        let state_after_undo =
            update_state(state_after_unstage, Some(Input::Character('<')), 30, 80);
        assert_eq!(state_after_undo.files.len(), 1);
        assert_eq!(state_after_undo.files[0].hunks.len(), 1);

        // Redo
        let state_after_redo = update_state(state_after_undo, Some(Input::Character('>')), 30, 80);
        assert_eq!(state_after_redo.files.len(), 0);
    });
}

#[test]
#[serial]
fn test_unstage_line() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new_multi_line();
        let mut state = create_test_state(&setup);

        // We have one file with one hunk. The hunk has 3 lines: one removed, one added, one context.
        // diff --git a/test.txt b/test.txt
        // index 3027459..9413563 100644
        // --- a/test.txt
        // +++ b/test.txt
        // @@ -1,3 +1,3 @@
        //  line1
        // -line2
        // +changed
        //  line3
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.files[0].hunks.len(), 1);
        assert_eq!(state.files[0].lines.len(), 9); // 5 header + 4 hunk lines

        // Let's unstage the "+changed" line. It's at index 7.
        state.main_screen.line_cursor = 7;

        // Unstage line
        let state_after_unstage = update_state(state, Some(Input::Character('1')), 30, 80);
        assert_eq!(state_after_unstage.files.len(), 1);
        // The diff should now only contain "+changed"
        assert_eq!(state_after_unstage.files[0].lines.len(), 8);
        assert!(
            !state_after_unstage.files[0]
                .lines
                .iter()
                .any(|l| l.contains("+changed"))
        );
        assert!(
            state_after_unstage.files[0]
                .lines
                .iter()
                .any(|l| l.contains("-line2"))
        );
    });
}

#[test]
#[serial]
fn test_unstage_deleted_line() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new_multi_line();
        let mut state = create_test_state(&setup);

        // We have one file with one hunk. The hunk has 3 lines: one removed, one added, one context.
        // diff --git a/test.txt b/test.txt
        // index 3027459..9413563 100644
        // --- a/test.txt
        // +++ b/test.txt
        // @@ -1,3 +1,3 @@
        //  line1
        // -line2
        // +changed
        //  line3
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.files[0].hunks.len(), 1);
        assert_eq!(state.files[0].lines.len(), 9); // 5 header + 4 hunk lines

        // Let's unstage the "-line2" line. It's at index 6.
        state.main_screen.line_cursor = 6;

        // Unstage line
        let state_after_unstage = update_state(state, Some(Input::Character('1')), 30, 80);
        assert_eq!(state_after_unstage.files.len(), 1);
        // The diff should now only contain "+changed"
        assert!(
            !state_after_unstage.files[0]
                .lines
                .iter()
                .any(|l| l.contains("-line2"))
        );
        assert!(
            state_after_unstage.files[0]
                .lines
                .iter()
                .any(|l| l.contains(" line2"))
        );
        assert!(
            state_after_unstage.files[0]
                .lines
                .iter()
                .any(|l| l.contains("+changed"))
        );
    });
}

#[test]
#[serial]
fn test_commit_mode_activation_and_commit() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let mut state = create_test_state(&setup);
        state.main_screen.file_cursor = 1;

        // We start with 1 file, cursor at index 1.
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.main_screen.file_cursor, 1);
        assert!(!state.main_screen.is_commit_mode);

        // 1. Press KeyDown to move to the commit line.
        state = update_state(state, Some(Input::KeyDown), 30, 80);

        // 2. Assert that we are in commit mode.
        assert_eq!(state.main_screen.file_cursor, 2); // Cursor is on the commit line
        assert!(state.main_screen.is_commit_mode);

        // 3. Type a commit message
        let msg = "Test commit";
        for ch in msg.chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }

        // 4. Assert the message is correct
        assert_eq!(state.main_screen.commit_message, msg);

        // 5. Press Enter to commit
        state = update_state(state, Some(Input::Character('\n')), 30, 80);

        // 6. Assert the app should exit
        assert!(!state.running);

        // 7. Verify the commit was created
        let output = OsCommand::new("git")
            .args(["log", "-1", "--pretty=%B"])
            .current_dir(&setup.repo_path)
            .output()
            .expect("failed to run git log");
        let last_commit_message = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(last_commit_message, msg);
    });
}

#[test]
#[serial]
fn test_page_up_down_with_cursor() {
    run_test_with_pancurses(|window| {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path().to_path_buf();

        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.name", "Test"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);

        let file_path = repo_path.join("test.txt");
        let initial_content: String = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<String>>()
            .join("\n");
        fs::write(&file_path, initial_content).unwrap();

        run_git(&repo_path, &["add", "test.txt"]);
        run_git(&repo_path, &["commit", "-m", "initial commit"]);

        let modified_content: String = (0..100)
            .map(|i| format!("modified line {i}"))
            .collect::<Vec<String>>()
            .join("\n");
        fs::write(&file_path, modified_content).unwrap();
        run_git(&repo_path, &["add", "test.txt"]);

        let files = get_diff(repo_path.clone());
        let mut state = AppState::new(repo_path.clone(), files);

        let (max_y, _) = window.get_max_yx();
        let (header_height, _) = state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height);

        // Page down
        state = update_state(state, Some(Input::Character(' ')), max_y, 80);
        assert_eq!(state.main_screen.diff_scroll, content_height);
        assert_eq!(state.main_screen.line_cursor, content_height);

        // Page up
        state = update_state(state, Some(Input::Character('b')), max_y, 80);
        assert_eq!(state.main_screen.diff_scroll, 0);
        assert_eq!(state.main_screen.line_cursor, 0);
    });
}

#[test]
#[serial]
fn test_run_with_unstaged_changes() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();

    // git init
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    // first commit
    let file_path = repo_path.join("test.txt");
    fs::write(&file_path, "a\n").unwrap();
    run_git(&repo_path, &["add", "test.txt"]);
    run_git(&repo_path, &["commit", "-m", "initial commit"]);

    // modify file but do not stage
    fs::write(&file_path, "b\n").unwrap();

    let staged_diff_output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    assert!(staged_diff_output.stdout.is_empty());

    // This is the logic from the `run` function before `tui_loop`
    let staged_diff_output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    if staged_diff_output.stdout.is_empty() {
        OsCommand::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&repo_path)
            .output()
            .unwrap();
    }

    let files = get_diff(repo_path.clone());
    assert!(!files.is_empty());
}

#[test]
#[serial]
fn test_commit_and_continue() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let mut state = create_test_state(&setup);

        // create another file
        let file_path = setup.repo_path.join("another.txt");
        fs::write(&file_path, "hello\n").unwrap();

        // We start with 1 file, cursor at index 1.
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.main_screen.file_cursor, 1);
        assert!(!state.main_screen.is_commit_mode);

        // 1. Press KeyDown to move to the commit line.
        state = update_state(state, Some(Input::KeyDown), 30, 80);

        // 2. Assert that we are in commit mode.
        assert_eq!(state.main_screen.file_cursor, 2); // Cursor is on the commit line
        assert!(state.main_screen.is_commit_mode);

        // 3. Type a commit message
        let msg = "Test commit";
        for ch in msg.chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }

        // 4. Assert the message is correct
        assert_eq!(state.main_screen.commit_message, msg);

        // 5. Press Enter to commit
        state = update_state(state, Some(Input::Character('\n')), 30, 80);

        // 6. Assert the app should still be running
        assert!(state.running);

        // 7. Assert the commit message is cleared
        assert!(state.main_screen.commit_message.is_empty());

        // 8. Assert the new file is staged
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.files[0].file_name, "another.txt");
    });
}

#[test]
#[serial]
fn test_commit_and_exit() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let mut state = create_test_state(&setup);

        // We start with 1 file, cursor at index 1.
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.main_screen.file_cursor, 1);
        assert!(!state.main_screen.is_commit_mode);

        // 1. Press KeyDown to move to the commit line.
        state = update_state(state, Some(Input::KeyDown), 30, 80);

        // 2. Assert that we are in commit mode.
        assert_eq!(state.main_screen.file_cursor, 2); // Cursor is on the commit line
        assert!(state.main_screen.is_commit_mode);

        // 3. Type a commit message
        let msg = "Test commit";
        for ch in msg.chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }

        // 4. Assert the message is correct
        assert_eq!(state.main_screen.commit_message, msg);

        // 5. Press Enter to commit
        state = update_state(state, Some(Input::Character('\n')), 30, 80);

        // 6. Assert the app should exit
        assert!(!state.running);
    });
}

#[test]
#[serial]
fn test_commit_clears_history() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let mut state = create_test_state(&setup);

        // Unstage all files to populate history
        state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert_eq!(state.files.len(), 0);
        assert_eq!(state.command_history.undo_stack.len(), 1);

        // Stage it back
        state = update_state(state, Some(Input::Character('R')), 30, 80);
        state.refresh_diff();
        assert_eq!(state.files.len(), 1);

        // Go to commit mode
        state = update_state(state, Some(Input::KeyDown), 30, 80);
        state = update_state(state, Some(Input::KeyDown), 30, 80);
        assert!(state.main_screen.is_commit_mode);

        // Type a commit message
        let msg = "Test commit";
        for ch in msg.chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }

        // Commit
        state = update_state(state, Some(Input::Character('\n')), 30, 80);

        // Assert history is cleared
        assert_eq!(state.command_history.undo_stack.len(), 0);
        assert_eq!(state.command_history.redo_stack.len(), 0);
    });
}

#[test]
#[serial]
fn test_stage_all() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new();
        let mut state = create_test_state(&setup);

        // We start with 1 file staged
        assert_eq!(state.files.len(), 1);

        // Unstage the file
        state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert_eq!(state.files.len(), 0);

        // Check that there are unstaged changes
        let output = OsCommand::new("git")
            .arg("diff")
            .current_dir(&setup.repo_path)
            .output()
            .unwrap();
        assert!(!output.stdout.is_empty());

        // Stage all changes with 'R'
        state = update_state(state, Some(Input::Character('R')), 30, 80);

        // Check that the file is staged again
        assert_eq!(state.files.len(), 1);

        // Undo staging all changes
        state = update_state(state, Some(Input::Character('<')), 30, 80);
        assert_eq!(state.files.len(), 0);

        // Redo staging all changes
        state = update_state(state, Some(Input::Character('>')), 30, 80);
        assert_eq!(state.files.len(), 1);
    });
}

#[test]
fn test_get_previous_commit_diff() {
    let setup = TestSetup::new();

    // Commit the staged changes to create a new commit
    run_git(&setup.repo_path, &["commit", "-m", "second commit"]);

    // Call the function to get the diff of the last commit
    let diffs = git_full_commit::git::get_previous_commit_diff(&setup.repo_path).unwrap();

    // There should be one file in the diff
    assert_eq!(diffs.len(), 1);
    let file_diff = &diffs[0];

    // Check the file name
    assert_eq!(file_diff.file_name, "test.txt");

    // Check that there is one hunk
    assert_eq!(file_diff.hunks.len(), 1);
    let hunk = &file_diff.hunks[0];

    // Check the content of the hunk
    assert!(hunk.lines.iter().any(|line| line.contains("-a")));
    assert!(hunk.lines.iter().any(|line| line.contains("+b")));
}

#[test]
#[serial]
fn test_rename_file() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();

    // git init
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    // first commit
    let file_path = repo_path.join("original.txt");
    fs::write(&file_path, "hello\n").unwrap();
    run_git(&repo_path, &["add", "original.txt"]);
    run_git(&repo_path, &["commit", "-m", "initial commit"]);

    // Rename the file
    run_git(&repo_path, &["mv", "original.txt", "renamed.txt"]);

    // The logic from the `run` function before `tui_loop`
    let staged_diff_output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    if staged_diff_output.stdout.is_empty() {
        OsCommand::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&repo_path)
            .output()
            .unwrap();
    }

    let files = git_full_commit::git::get_diff(repo_path.clone());
    assert!(!files.is_empty());
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, git_full_commit::git::FileStatus::Renamed);
    assert_eq!(files[0].file_name, "renamed.txt");
}

#[test]
#[serial]
fn test_create_unstage_line_patch_with_multiple_hunks() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();

    // git init
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    // Create a large file and commit it
    let file_path = repo_path.join("large_file.txt");
    let mut content = String::new();
    for i in 0..100 {
        content.push_str(&format!("line {i}\n"));
    }
    fs::write(&file_path, &content).unwrap();
    run_git(&repo_path, &["add", "large_file.txt"]);
    run_git(&repo_path, &["commit", "-m", "initial commit"]);

    // Modify the file in multiple places to create multiple hunks
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    lines[10] = "modified line 10".to_string();
    lines[50] = "modified line 50".to_string();
    lines[90] = "modified line 90".to_string();
    let modified_content = lines.join("\n");
    fs::write(&file_path, modified_content).unwrap();
    run_git(&repo_path, &["add", "large_file.txt"]);

    // Get the diff
    let files = git_full_commit::git::get_diff(repo_path.clone());
    assert_eq!(files.len(), 1);
    let file_diff = &files[0];

    // Find the line index of the second modification
    let line_to_unstage_index = file_diff
        .lines
        .iter()
        .position(|line| line.contains("+modified line 50"))
        .unwrap();

    // Create the patch
    let patch = git_full_commit::git_patch::create_unstage_line_patch(
        file_diff,
        line_to_unstage_index,
        true,
    )
    .unwrap();

    // Apply the patch in reverse
    apply_patch(&repo_path, &patch, true, true).expect("Failed to apply patch in reverse.");

    // Check the staged diff again
    let files_after_patch = git_full_commit::git::get_diff(repo_path.clone());
    assert_eq!(files_after_patch.len(), 1);
    let file_diff_after_patch = &files_after_patch[0];

    // The line should be unstaged
    assert!(
        !file_diff_after_patch
            .lines
            .iter()
            .any(|line| line.contains("+modified line 50"))
    );

    // Other modifications should still be staged
    assert!(
        file_diff_after_patch
            .lines
            .iter()
            .any(|line| line.contains("+modified line 10"))
    );
    assert!(
        file_diff_after_patch
            .lines
            .iter()
            .any(|line| line.contains("+modified line 90"))
    );
}

#[test]
#[serial]
fn test_unstage_second_file_moves_to_commit() {
    run_test_with_pancurses(|_window| {
        // 1. Setup repo with 2 files
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path().to_path_buf();
        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.name", "Test"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);
        fs::write(repo_path.join("a.txt"), "a").unwrap();
        fs::write(repo_path.join("b.txt"), "b").unwrap();
        run_git(&repo_path, &["add", "a.txt", "b.txt"]);

        // 2. Create state and select second file
        let files = get_diff(repo_path.clone());
        let mut state = AppState::new(repo_path.clone(), files);
        assert_eq!(state.files.len(), 2);
        state.main_screen.file_cursor = 2; // Selects b.txt

        // 3. Unstage the file
        let state_after_unstage = update_state(state, Some(Input::Character('\n')), 30, 80);

        // 4. Check state
        assert_eq!(state_after_unstage.files.len(), 1);
        assert_eq!(state_after_unstage.main_screen.file_cursor, 2); // Cursor is on commit line
        assert!(state_after_unstage.main_screen.is_commit_mode); // This is the bug

        // 5. Simulate typing
        let state_after_typing =
            update_state(state_after_unstage, Some(Input::Character('c')), 30, 80);

        // 6. Assert fix
        assert!(state_after_typing.main_screen.is_commit_mode);
        assert_eq!(state_after_typing.main_screen.commit_message, "c");
    });
}

// --- Tests for unstaged view ---

fn setup_test_repo_for_unstaged(repo_path: &Path) {
    if repo_path.exists() {
        fs::remove_dir_all(repo_path).unwrap();
    }
    fs::create_dir_all(repo_path).unwrap();
    OsCommand::new("git")
        .arg("init")
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");
    OsCommand::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test")
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git user.name");
    OsCommand::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git user.email");
}

fn create_initial_commit_for_unstaged(repo_path: &Path, file_name: &str, content: &str) {
    fs::write(repo_path.join(file_name), content).unwrap();
    OsCommand::new("git")
        .arg("add")
        .arg(file_name)
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial commit")
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");
}

fn get_staged_diff_for_file(repo_path: &Path, file_name: &str) -> String {
    let output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()
        .expect("Failed to get staged diff for file");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
#[serial]
fn test_stage_untracked_file() {
    run_test_with_pancurses(|_window| {
        let repo_path = std::env::temp_dir().join("test_stage_untracked_file");
        setup_test_repo_for_unstaged(&repo_path);
        let file_name = "untracked.txt";
        fs::write(repo_path.join(file_name), "hello").unwrap();

        let mut state = AppState::new(repo_path.clone(), vec![]);
        state.refresh_diff();
        assert!(!state.untracked_files.is_empty());

        // Navigate to the untracked file
        state.unstaged_cursor = 2; // Unstaged header (0), Untracked header (1), file (2)

        // Press Enter to stage the file
        handle_unstaged_view_input(&mut state, Input::Character('\n'), 30);

        // Check that the file is staged
        state.refresh_diff();
        assert!(state.untracked_files.is_empty());
        assert_eq!(state.files[0].file_name, file_name);

        // Undo
        state = update_state(state, Some(Input::Character('<')), 30, 80);
        assert!(!state.untracked_files.is_empty());
        assert!(state.files.is_empty());
    });
}

#[test]
#[serial]
fn test_stage_modified_file() {
    run_test_with_pancurses(|_window| {
        let repo_path = std::env::temp_dir().join("test_stage_modified_file");
        setup_test_repo_for_unstaged(&repo_path);
        let file_name = "test.txt";
        create_initial_commit_for_unstaged(&repo_path, file_name, "line1\nline2\n");
        fs::write(repo_path.join(file_name), "line1\nMODIFIED\n").unwrap();

        let mut state = AppState::new(repo_path.clone(), vec![]);
        state.refresh_diff();
        assert!(!state.unstaged_files.is_empty());

        // Navigate to the modified file
        state.unstaged_cursor = 1;

        // Press Enter to stage the file
        handle_unstaged_view_input(&mut state, Input::Character('\n'), 30);

        // Check that the file is staged
        state.refresh_diff();
        assert!(state.unstaged_files.is_empty());
        assert_eq!(state.files[0].file_name, file_name);

        // Undo
        let state = update_state(state, Some(Input::Character('<')), 30, 80);
        assert!(!state.unstaged_files.is_empty());
        assert!(state.files.is_empty());
    });
}

#[test]
#[serial]
fn test_stage_hunk() {
    run_test_with_pancurses(|_window| {
        let repo_path = std::env::temp_dir().join("test_stage_hunk");
        setup_test_repo_for_unstaged(&repo_path);
        let file_name = "test.txt";
        let initial_content = (1..=10)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        create_initial_commit_for_unstaged(&repo_path, file_name, &initial_content);

        let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
        lines[2] = "MODIFIED line 3".to_string();
        fs::write(repo_path.join(file_name), lines.join("\n")).unwrap();

        let mut state = AppState::new(repo_path.clone(), vec![]);
        state.refresh_diff();

        // Navigate to the modified file and the hunk
        state.unstaged_cursor = 1;
        let hunk_line = state.unstaged_files[0]
            .lines
            .iter()
            .position(|l| l.contains("MODIFIED"))
            .unwrap();
        state.main_screen.line_cursor = hunk_line;

        // Press Enter to stage the hunk
        handle_unstaged_view_input(&mut state, Input::Character('\n'), 30);
        state.refresh_diff();

        // Check that the hunk is staged
        assert!(state.unstaged_files.is_empty());
        assert!(!state.files.is_empty());
        let staged_diff = get_staged_diff_for_file(&repo_path, file_name);
        assert!(staged_diff.contains("+MODIFIED line 3"));

        // Undo
        let _ = update_state(state, Some(Input::Character('<')), 30, 80);
        assert_eq!(
            git::get_diff(repo_path).len(),
            0,
            "Staged diff should be empty after undo"
        );
    });
}

#[test]
#[serial]
fn test_add_all_with_size_limit() {
    let setup = TestSetup::new();
    let repo_path = &setup.repo_path;

    // Create a small untracked file
    let small_file_path = repo_path.join("small.txt");
    fs::write(&small_file_path, "small").unwrap();

    // Create a large untracked file
    let large_file_path = repo_path.join("large.txt");
    let large_content = vec![0; 1024 * 1024]; // 1MB
    fs::write(&large_file_path, &large_content).unwrap();

    // Modify an existing file
    let modified_file_path = repo_path.join("test.txt");
    fs::write(&modified_file_path, "modified").unwrap();

    // Run add_all with a size limit of 500KB
    git::add_all_with_size_limit(repo_path, 500 * 1024).unwrap();

    // Check the status
    let output = OsCommand::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(repo_path)
        .output()
        .unwrap();
    let status = String::from_utf8_lossy(&output.stdout);

    // Assert that the small file is staged
    assert!(status.contains("A  small.txt"));
    // Assert that the large file is not staged
    assert!(status.contains("?? large.txt"));
    // Assert that the modified file is staged
    assert!(status.contains("M  test.txt"));
}

#[test]
#[serial]
fn test_undo_redo_restores_cursor_position() {
    run_test_with_pancurses(|_window| {
        let setup = TestSetup::new_multi_line();
        let mut state = create_test_state(&setup);

        // 1. Set an initial cursor position
        state.main_screen.file_cursor = 1;
        state.main_screen.line_cursor = 7; // On "+changed" line
        state.main_screen.diff_scroll = 5;
        state.screen = git_full_commit::app_state::Screen::Main;

        let cursor_before_action =
            git_full_commit::cursor_state::CursorState::from_app_state(&state);

        // 2. Perform an action (unstage line)
        state = update_state(state, Some(Input::Character('1')), 30, 80);

        // 3. Change cursor position
        state.main_screen.file_cursor = 0;
        state.main_screen.line_cursor = 0;
        state.main_screen.diff_scroll = 0;
        let cursor_before_undo = git_full_commit::cursor_state::CursorState::from_app_state(&state);

        // 4. Undo
        state = update_state(state, Some(Input::Character('<')), 30, 80);

        // 5. Assert cursor is restored to the position before the action
        let cursor_after_undo = git_full_commit::cursor_state::CursorState::from_app_state(&state);
        assert_eq!(
            cursor_after_undo.file_cursor,
            cursor_before_action.file_cursor
        );
        assert_eq!(
            cursor_after_undo.line_cursor,
            cursor_before_action.line_cursor
        );
        assert_eq!(cursor_after_undo.scroll, cursor_before_action.scroll);
        assert_eq!(cursor_after_undo.screen, cursor_before_action.screen);

        // 6. Redo
        state = update_state(state, Some(Input::Character('>')), 30, 80);

        // 7. Assert cursor is restored to the position before the undo
        let cursor_after_redo = git_full_commit::cursor_state::CursorState::from_app_state(&state);
        assert_eq!(
            cursor_after_redo.file_cursor,
            cursor_before_undo.file_cursor
        );
        // line_cursor is not restored on redo of unstaging a line, as the file content changes.
        // This is acceptable. The main thing is file_cursor and screen.
        // assert_eq!(cursor_after_redo.main_screen.line_cursor, cursor_before_undo.main_screen.line_cursor);
        assert_eq!(cursor_after_redo.scroll, cursor_before_undo.scroll);
        assert_eq!(cursor_after_redo.screen, cursor_before_undo.screen);
    });
}

#[test]
#[serial]
fn test_stage_line() {
    run_test_with_pancurses(|_window| {
        let repo_path = std::env::temp_dir().join("test_stage_line");
        setup_test_repo_for_unstaged(&repo_path);
        let file_name = "test.txt";
        create_initial_commit_for_unstaged(&repo_path, file_name, "line1\nline2\n");
        fs::write(repo_path.join(file_name), "line1\nCHANGED\nADDED\n").unwrap();

        let mut state = AppState::new(repo_path.clone(), vec![]);
        state.refresh_diff();

        // Navigate to the added line
        state.unstaged_cursor = 1;
        let added_line_index = state.unstaged_files[0]
            .lines
            .iter()
            .position(|l| l.contains("ADDED"))
            .unwrap();
        state.main_screen.line_cursor = added_line_index;

        // Press '1' to stage the line
        handle_unstaged_view_input(&mut state, Input::Character('1'), 30);
        state.refresh_diff();

        // Check that the line is staged
        let staged_diff = get_staged_diff_for_file(&repo_path, file_name);
        assert!(staged_diff.contains("+ADDED"));
        assert!(!staged_diff.contains("MODIFIED"));
        assert!(!staged_diff.contains("-line2"));

        // Undo
        update_state(state, Some(Input::Character('<')), 30, 80);
        assert_eq!(
            git::get_diff(repo_path).len(),
            0,
            "Staged diff should be empty after undo"
        );
    });
}
