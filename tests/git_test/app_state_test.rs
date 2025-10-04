use super::common::{run_test_with_pancurses, TestRepo};
use git_full_commit::app_state::{AppState, Screen};
use git_full_commit::git::get_diff;
use git_full_commit::ui::update::update_state;
use pancurses::Input;
use serial_test::serial;
use std::path::PathBuf;
use std::process::Command as OsCommand;

// This helper is specific to this file, so it stays here.
fn create_test_state(repo_path: PathBuf) -> AppState {
    let files = get_diff(repo_path.clone());
    AppState::new(repo_path, files)
}

#[test]
#[serial]
fn test_update_state_quit() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        let state = create_test_state(repo.path);
        let new_state = update_state(state, Some(Input::Character('\u{3}')), 30, 80);
        assert!(!new_state.running);
    });
}

#[test]
#[serial]
fn test_unstage_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "a\n");
        repo.add_file("test.txt");
        let state = create_test_state(repo.path);
        let new_state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert_eq!(new_state.files.len(), 0);
    });
}

#[test]
#[serial]
fn test_unstage_hunk_by_line_with_undo_redo() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "a\n");
        repo.add_file("test.txt");
        repo.commit("initial commit");
        repo.create_file("test.txt", "b\n");
        repo.add_file("test.txt");

        let mut state = create_test_state(repo.path.clone());

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
        let repo = TestRepo::new();
        repo.create_file("test.txt", "line1\nline2\nline3\n");
        repo.add_file("test.txt");
        repo.commit("initial");
        repo.create_file("test.txt", "line1\nchanged\nline3\n");
        repo.add_file("test.txt");

        let mut state = create_test_state(repo.path.clone());

        assert_eq!(state.files.len(), 1);
        assert_eq!(state.files[0].hunks.len(), 1);

        // Let's unstage the "+changed" line.
        let line_index = state.files[0]
            .lines
            .iter()
            .position(|l| l.contains("+changed"))
            .unwrap();
        state.main_screen.line_cursor = line_index;

        // Unstage line
        let state_after_unstage = update_state(state, Some(Input::Character('1')), 30, 80);
        assert_eq!(state_after_unstage.files.len(), 1);

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
fn test_commit_mode_activation_and_commit() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "a");
        repo.add_file("a.txt");
        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 1;

        assert!(!state.main_screen.is_commit_mode);

        state = update_state(state, Some(Input::KeyDown), 30, 80);
        assert!(state.main_screen.is_commit_mode);

        let msg = "Test commit";
        for ch in msg.chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }
        assert_eq!(state.main_screen.commit_message, msg);

        state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert!(!state.running);

        let output = OsCommand::new("git")
            .args(["log", "-1", "--pretty=%B"])
            .current_dir(&repo.path)
            .output()
            .expect("failed to run git log");
        let last_commit_message = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(last_commit_message, msg);
    });
}

#[test]
#[serial]
fn test_ignore_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "initial content");
        repo.add_all();
        repo.commit("initial commit");

        let file_to_ignore = "some_file.txt";
        repo.create_file(file_to_ignore, "Hello");
        repo.add_all();

        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 1; // Select the file

        let mut updated_state = update_state(state, Some(Input::Character('i')), 80, 80);

        let gitignore_path = repo.path.join(".gitignore");
        assert!(gitignore_path.exists(), ".gitignore should be created");
        let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
        assert!(
            gitignore_content.contains(file_to_ignore),
            ".gitignore should contain the ignored file"
        );

        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should only contain .gitignore"
        );
        assert_eq!(
            updated_state.files[0].file_name, ".gitignore",
            "The remaining file should be .gitignore"
        );

        updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should contain the original file again"
        );
        assert_eq!(
            updated_state.files[0].file_name, file_to_ignore,
            "The file should be the one we ignored"
        );
    });
}

#[test]
#[serial]
fn test_discard_hunk_in_staged() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        let initial_content = (1..=20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        repo.create_file("test.txt", &initial_content);
        repo.add_all();
        repo.commit("initial commit");

        let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
        lines[2] = "modified line 3".to_string();
        lines[15] = "modified line 16".to_string();
        let modified_content = lines.join("\n");
        repo.create_file("test.txt", &modified_content);
        repo.add_all();

        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 1; // Select the file
        state.main_screen.is_diff_cursor_active = true;
        let line_in_diff = state.files[0]
            .lines
            .iter()
            .position(|l| l.contains("modified line 16"))
            .unwrap_or(15);
        state.main_screen.line_cursor = line_in_diff;

        let updated_state = update_state(state, Some(Input::Character('!')), 80, 80);

        let staged_diff = repo.get_status();
        assert!(!staged_diff.contains("modified line 16"));
        assert!(staged_diff.contains("M  test.txt"));

        let working_diff = git_full_commit::git::get_unstaged_diff(&repo.path);
        assert!(
            working_diff.is_empty(),
            "Working directory should be clean"
        );

        let _updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

        let staged_diff_after_undo = repo.get_status();
        assert!(staged_diff_after_undo.contains("M  test.txt"));
    });
}

#[test]
#[serial]
fn test_stage_all_and_undo() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("committed.txt", "initial content");
        repo.add_all();
        repo.commit("initial commit");

        repo.create_file("committed.txt", "modified content");
        repo.create_file("untracked.txt", "new file");

        let files = git_full_commit::git::get_unstaged_diff(&repo.path);
        let state = AppState::new(repo.path.clone(), files);

        let updated_state = update_state(state, Some(Input::Character('R')), 80, 80);

        let status_str = repo.get_status();
        assert!(status_str.contains("M  committed.txt"));
        assert!(status_str.contains("A  untracked.txt"));

        let _updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);

        let status_str_after_undo = repo.get_status();
        assert!(status_str_after_undo.contains(" M committed.txt"));
        assert!(status_str_after_undo.contains("?? untracked.txt"));
    });
}

#[test]
#[serial]
fn test_unstage_all() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("committed.txt", "a\n");
        repo.add_all();
        repo.commit("initial");

        repo.create_file("committed.txt", "b\n");
        repo.create_file("new.txt", "c\n");
        repo.add_all();

        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 0; // Select "Staged changes" header

        let state = update_state(state, Some(Input::Character('\n')), 80, 80);
        let status = repo.get_status();
        assert!(status.contains(" M committed.txt"));
        assert!(status.contains("?? new.txt"));

        assert_eq!(state.main_screen.file_cursor, 0);
        let state = update_state(state, Some(Input::Character('<')), 80, 80);
        let status = repo.get_status();
        assert!(status.contains("M  committed.txt"));
        assert!(status.contains("A  new.txt"));

        let _ = update_state(state, Some(Input::Character('>')), 80, 80);
        let status = repo.get_status();
        assert!(status.contains(" M committed.txt"));
        assert!(status.contains("?? new.txt"));
    });
}

#[test]
#[serial]
fn test_discard_unstaged_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "line1\nline2\n");
        repo.add_all();
        repo.commit("initial");
        repo.create_file("test.txt", "line1\nMODIFIED\n");

        let mut state = AppState::new(repo.path.clone(), vec![]);
        state.refresh_diff();
        state.screen = Screen::Unstaged;
        state.unstaged_screen.unstaged_cursor = 1;

        let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
        let status = repo.get_status();
        assert!(status.is_empty(), "Git status should be clean");
        assert!(state_after_discard.unstaged_screen.unstaged_files.is_empty());

        let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
        let status_after_undo = repo.get_status();
        assert!(status_after_undo.contains(" M test.txt"));
        assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);
    });
}