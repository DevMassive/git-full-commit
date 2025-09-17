use git_reset_pp::*;
use pancurses::{initscr, Input, Window};
use std::fs;
use std::path::PathBuf;
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
    let (files, lines) = get_diff(test_setup.repo_path.clone());
    AppState::new(test_setup.repo_path.clone(), files, lines)
}

fn setup_pancurses() -> Window {
    let window = initscr();
    window.keypad(true);
    pancurses::noecho();
    window
}

#[test]
fn test_update_state_quit() {
    let setup = TestSetup::new();
    let window = setup_pancurses();
    let state = create_test_state(&setup);
    let new_state = update_state(state, Some(Input::Character('q')), &window);
    assert!(!new_state.running);
}

#[test]
fn test_unstage_file() {
    let setup = TestSetup::new();
    let window = setup_pancurses();
    let mut state = create_test_state(&setup);
    state.cursor_level = CursorLevel::File;
    let new_state = update_state(state, Some(Input::Character('\n')), &window);
    assert_eq!(new_state.files.len(), 0);
}

#[test]
fn test_unstage_hunk_with_undo_redo() {
    let setup = TestSetup::new();
    let window = setup_pancurses();
    let mut state = create_test_state(&setup);

    // Ensure we have a file with a hunk
    assert_eq!(state.files.len(), 1);
    assert_eq!(state.files[0].hunks.len(), 1);

    // Navigate to hunk level
    state.cursor_level = CursorLevel::Hunk;

    // Unstage hunk
    let state_after_unstage = update_state(state, Some(Input::Character('\n')), &window);
    assert_eq!(state_after_unstage.files.len(), 0);

    // Undo
    let state_after_undo = update_state(state_after_unstage, Some(Input::Character('u')), &window);
    assert_eq!(state_after_undo.files.len(), 1);
    assert_eq!(state_after_undo.files[0].hunks.len(), 1);

    // Redo
    let state_after_redo = update_state(state_after_undo, Some(Input::Character('r')), &window);
    assert_eq!(state_after_redo.files.len(), 0);
}

#[test]
fn test_hunk_half_page_scroll() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();

    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    let file_path = repo_path.join("test.txt");
    let initial_content: String = (0..100).map(|i| format!("line {}", i)).collect::<Vec<String>>().join("\n");
    fs::write(&file_path, initial_content).unwrap();

    run_git(&repo_path, &["add", "test.txt"]);
    run_git(&repo_path, &["commit", "-m", "initial commit"]);

    let modified_content: String = (0..100).map(|i| format!("modified line {}", i)).collect::<Vec<String>>().join("\n");
    fs::write(&file_path, modified_content).unwrap();
    run_git(&repo_path, &["add", "test.txt"]);

    let (files, lines) = get_diff(repo_path.clone());
    let mut state = AppState::new(repo_path.clone(), files, lines);
    state.cursor_level = CursorLevel::Hunk;

    let window = setup_pancurses();
    let (max_y, _) = window.get_max_yx();
    let window_height = max_y as usize;

    let state_after_scroll_down = update_state(state, Some(Input::KeyDown), &window);
    assert_eq!(state_after_scroll_down.scroll, window_height / 2);

    let state_after_scroll_up = update_state(state_after_scroll_down, Some(Input::KeyUp), &window);
    assert_eq!(state_after_scroll_up.scroll, 0);
}