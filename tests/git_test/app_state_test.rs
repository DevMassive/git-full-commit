use super::common::{run_test_with_pancurses, TestRepo};
use git_full_commit::app_state::{AppState, Screen};
use git_full_commit::cursor_state::CursorState;
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;
use serial_test::serial;
use std::path::PathBuf;
use std::process::Command as OsCommand;

fn create_test_state(repo_path: PathBuf) -> AppState {
    let files = git::get_diff(repo_path.clone());
    AppState::new(repo_path, files)
}

#[test]
#[serial]
fn test_update_state_quit() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        let state = create_test_state(repo.path.clone());
        let new_state = update_state(state, Some(Input::Character('\u{3}')), 30, 80);
        assert!(!new_state.running);
    });
}

#[test]
#[serial]
fn test_unstage_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "a");
        repo.add_file("test.txt");
        let state = create_test_state(repo.path.clone());
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
        repo.commit("initial");
        repo.append_file("test.txt", "b\n");
        repo.add_file("test.txt");

        let mut state = create_test_state(repo.path.clone());

        let hunk = &state.files[0].hunks[0].clone();
        state.main_screen.line_cursor = hunk.start_line + 1;

        let state_after_unstage = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert_eq!(state_after_unstage.files.len(), 0);

        let state_after_undo =
            update_state(state_after_unstage, Some(Input::Character('<')), 30, 80);
        assert_eq!(state_after_undo.files.len(), 1);

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
        let line_index = state.files[0]
            .lines
            .iter()
            .position(|l| l.contains("+changed"))
            .unwrap();
        state.main_screen.line_cursor = line_index;

        let state_after_unstage = update_state(state, Some(Input::Character('1')), 30, 80);
        assert!(!state_after_unstage.files[0]
            .lines
            .iter()
            .any(|l| l.contains("+changed")));
    });
}

#[test]
#[serial]
fn test_unstage_deleted_line() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "line1\nline2\nline3\n");
        repo.add_file("test.txt");
        repo.commit("initial");
        repo.create_file("test.txt", "line1\nchanged\nline3\n");
        repo.add_file("test.txt");

        let mut state = create_test_state(repo.path.clone());
        let line_index = state.files[0]
            .lines
            .iter()
            .position(|l| l.contains("-line2"))
            .unwrap();
        state.main_screen.line_cursor = line_index;

        let state_after_unstage = update_state(state, Some(Input::Character('1')), 30, 80);
        assert!(!state_after_unstage.files[0]
            .lines
            .iter()
            .any(|l| l.contains("-line2")));
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

        state = update_state(state, Some(Input::KeyDown), 30, 80);
        assert!(state.main_screen.is_commit_mode);

        let msg = "Test commit";
        for ch in msg.chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }
        state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert!(!state.running);

        let output = OsCommand::new("git")
            .args(["log", "-1", "--pretty=%B"])
            .current_dir(&repo.path)
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert_eq!(log.trim(), msg);
    });
}

#[test]
#[serial]
fn test_page_up_down_with_cursor() {
    run_test_with_pancurses(|window| {
        let repo = TestRepo::new();
        let long_content: String = (0..100).map(|i| format!("line {i}\n")).collect();
        repo.create_file("test.txt", &long_content);
        repo.add_file("test.txt");
        repo.commit("initial");
        let modified_content: String = (0..100).map(|i| format!("modified {i}\n")).collect();
        repo.create_file("test.txt", &modified_content);
        repo.add_file("test.txt");

        let mut state = create_test_state(repo.path.clone());

        let (max_y, _) = window.get_max_yx();
        let (header_height, _) = state.main_header_height(max_y);
        let content_height = (max_y as usize).saturating_sub(header_height);

        state = update_state(state, Some(Input::Character(' ')), max_y, 80);
        assert_eq!(state.main_screen.diff_scroll, content_height);

        state = update_state(state, Some(Input::Character('b')), max_y, 80);
        assert_eq!(state.main_screen.diff_scroll, 0);
    });
}

#[test]
#[serial]
fn test_commit_and_continue() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "a");
        repo.add_file("a.txt");
        let mut state = create_test_state(repo.path.clone());
        repo.create_file("b.txt", "b");

        state = update_state(state, Some(Input::KeyDown), 30, 80); // To commit line
        state = update_state(state, Some(Input::KeyDown), 30, 80); // Enter commit mode
        for ch in "Test".chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }
        state = update_state(state, Some(Input::Character('\n')), 30, 80);

        assert!(state.running);
        assert!(state.main_screen.commit_message.is_empty());
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.files[0].file_name, "b.txt");
    });
}

#[test]
#[serial]
fn test_commit_and_exit() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "a");
        repo.add_file("a.txt");
        let mut state = create_test_state(repo.path.clone());

        state = update_state(state, Some(Input::KeyDown), 30, 80);
        state = update_state(state, Some(Input::KeyDown), 30, 80);
        for ch in "Test".chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }
        state = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert!(!state.running);
    });
}

#[test]
#[serial]
fn test_commit_clears_history() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "a");
        repo.add_file("a.txt");
        let mut state = create_test_state(repo.path.clone());

        state = update_state(state, Some(Input::Character('\n')), 30, 80); // Unstage
        assert_eq!(state.command_history.undo_stack.len(), 1);

        state = update_state(state, Some(Input::Character('R')), 30, 80); // Stage all
        state.refresh_diff();

        state = update_state(state, Some(Input::KeyDown), 30, 80);
        state = update_state(state, Some(Input::KeyDown), 30, 80);
        for ch in "Test".chars() {
            state = update_state(state, Some(Input::Character(ch)), 30, 80);
        }
        state = update_state(state, Some(Input::Character('\n')), 30, 80);

        assert_eq!(state.command_history.undo_stack.len(), 0);
        assert_eq!(state.command_history.redo_stack.len(), 0);
    });
}

#[test]
#[serial]
fn test_stage_all() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "a");
        repo.add_file("a.txt");
        let mut state = create_test_state(repo.path.clone());
        state = update_state(state, Some(Input::Character('\n')), 30, 80); // Unstage
        assert_eq!(state.files.len(), 0);

        state = update_state(state, Some(Input::Character('R')), 30, 80); // Stage all
        assert_eq!(state.files.len(), 1);

        state = update_state(state, Some(Input::Character('<')), 30, 80); // Undo
        assert_eq!(state.files.len(), 0);

        state = update_state(state, Some(Input::Character('>')), 30, 80); // Redo
        assert_eq!(state.files.len(), 1);
    });
}

#[test]
#[serial]
fn test_unstage_second_file_moves_to_commit() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("a.txt", "a");
        repo.create_file("b.txt", "b");
        repo.add_all();

        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 2;

        let state_after_unstage = update_state(state, Some(Input::Character('\n')), 30, 80);
        assert!(state_after_unstage.main_screen.is_commit_mode);
    });
}

#[test]
#[serial]
fn test_undo_redo_restores_cursor_position() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "line1\nline2\n");
        repo.add_file("test.txt");
        repo.commit("initial");
        repo.create_file("test.txt", "line1\nchanged\n");
        repo.add_file("test.txt");

        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 1;
        state.main_screen.line_cursor = 7;
        state.main_screen.diff_scroll = 5;
        let cursor_before_action = CursorState::from_app_state(&state);

        state = update_state(state, Some(Input::Character('1')), 30, 80); // Unstage line
        state.main_screen.line_cursor = 0;
        state.main_screen.diff_scroll = 0;

        state = update_state(state, Some(Input::Character('<')), 30, 80); // Undo
        let cursor_after_undo = CursorState::from_app_state(&state);
        assert_eq!(cursor_after_undo, cursor_before_action);
    });
}

// From git_actions.rs
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
        state.main_screen.file_cursor = 1;

        let mut updated_state = update_state(state, Some(Input::Character('i')), 80, 80);

        let gitignore_path = repo.path.join(".gitignore");
        assert!(gitignore_path.exists());
        let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
        assert!(gitignore_content.contains(file_to_ignore));

        assert_eq!(updated_state.files.len(), 1);
        assert_eq!(updated_state.files[0].file_name, ".gitignore");

        updated_state = update_state(updated_state, Some(Input::Character('<')), 80, 80);
        assert_eq!(updated_state.files.len(), 1);
        assert_eq!(updated_state.files[0].file_name, file_to_ignore);
    });
}

#[test]
#[serial]
fn test_discard_hunk() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        let initial_content = (1..=20).map(|i| format!("line {i}\n")).collect::<String>();
        repo.create_file("test.txt", &initial_content);
        repo.add_all();
        repo.commit("initial commit");

        let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
        lines[2] = "modified line 3".to_string();
        lines[15] = "modified line 16".to_string();
        repo.create_file("test.txt", &lines.join("\n"));
        repo.add_all();

        let mut state = create_test_state(repo.path.clone());
        state.main_screen.file_cursor = 1;
        state.main_screen.is_diff_cursor_active = true;
        let line_in_diff = state.files[0]
            .lines
            .iter()
            .position(|l| l.contains("modified line 16"))
            .unwrap();
        state.main_screen.line_cursor = line_in_diff;

        let updated_state = update_state(state, Some(Input::Character('!')), 80, 80);

        let staged_diff_str = repo.get_status();
        assert!(!staged_diff_str.contains("modified line 16"));
        assert!(staged_diff_str.contains("M  test.txt"));

        let _ = update_state(updated_state, Some(Input::Character('<')), 80, 80);
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

        let files = git::get_unstaged_diff(&repo.path);
        let state = AppState::new(repo.path.clone(), files);
        let updated_state = update_state(state, Some(Input::Character('R')), 80, 80);

        let status_str = repo.get_status();
        assert!(status_str.contains("M  committed.txt"));
        assert!(status_str.contains("A  untracked.txt"));

        let _ = update_state(updated_state, Some(Input::Character('<')), 80, 80);
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
        state.main_screen.file_cursor = 0;

        let state = update_state(state, Some(Input::Character('\n')), 80, 80);
        let status = repo.get_status();
        assert!(status.contains(" M committed.txt"));
        assert!(status.contains("?? new.txt"));

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
        assert!(status.is_empty());
        assert!(state_after_discard.unstaged_screen.unstaged_files.is_empty());

        let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
        let status_after_undo = repo.get_status();
        assert!(status_after_undo.contains(" M test.txt"));
        assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);
    });
}

#[test]
#[serial]
fn test_discard_untracked_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        let file_path = repo.path.join("untracked.txt");
        repo.create_file("untracked.txt", "hello");

        let mut state = AppState::new(repo.path.clone(), vec![]);
        state.refresh_diff();
        state.screen = Screen::Unstaged;
        state.unstaged_screen.unstaged_cursor = 2;

        let state_after_discard = update_state(state, Some(Input::Character('!')), 80, 80);
        assert!(!file_path.exists());
        assert!(state_after_discard.unstaged_screen.untracked_files.is_empty());

        let state_after_undo = update_state(state_after_discard, Some(Input::Character('<')), 80, 80);
        assert!(file_path.exists());
        assert_eq!(state_after_undo.unstaged_screen.untracked_files.len(), 1);
    });
}

#[test]
#[serial]
fn test_ignore_unstaged_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("test.txt", "initial");
        repo.add_all();
        repo.commit("initial");
        repo.create_file("test.txt", "modified");

        let mut state = AppState::new(repo.path.clone(), vec![]);
        state.refresh_diff();
        state.screen = Screen::Unstaged;
        state.unstaged_screen.unstaged_cursor = 1;

        let state_after_ignore = update_state(state, Some(Input::Character('i')), 80, 80);
        let gitignore_content = std::fs::read_to_string(repo.path.join(".gitignore")).unwrap();
        assert!(gitignore_content.contains("test.txt"));

        let state_after_undo = update_state(state_after_ignore, Some(Input::Character('<')), 80, 80);
        assert!(!repo.path.join(".gitignore").exists());
        assert_eq!(state_after_undo.unstaged_screen.unstaged_files.len(), 1);
    });
}

#[test]
#[serial]
fn test_ignore_untracked_file() {
    run_test_with_pancurses(|_window| {
        let repo = TestRepo::new();
        repo.create_file("untracked.txt", "hello");

        let mut state = AppState::new(repo.path.clone(), vec![]);
        state.refresh_diff();
        state.screen = Screen::Unstaged;
        state.unstaged_screen.unstaged_cursor = 2;

        let state_after_ignore = update_state(state, Some(Input::Character('i')), 80, 80);
        let gitignore_content = std::fs::read_to_string(repo.path.join(".gitignore")).unwrap();
        assert!(gitignore_content.contains("untracked.txt"));

        let state_after_undo = update_state(state_after_ignore, Some(Input::Character('<')), 80, 80);
        assert!(!repo.path.join(".gitignore").exists());
        assert_eq!(state_after_undo.unstaged_screen.untracked_files.len(), 1);
    });
}