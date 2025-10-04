use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::unstaged_screen::handle_input;
use git_full_commit::ui::update::update_state;
use pancurses::{endwin, initscr, Input, Window};
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::process::Command as OsCommand;

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
        assert!(!state.unstaged_screen.untracked_files.is_empty());

        // Navigate to the untracked file
        state.unstaged_screen.unstaged_cursor = 2; // Unstaged header (0), Untracked header (1), file (2)

        // Press Enter to stage the file
        handle_input(&mut state, Input::Character('\n'), 30);

        // Check that the file is staged
        state.refresh_diff();
        assert!(state.unstaged_screen.untracked_files.is_empty());
        assert_eq!(state.files[0].file_name, file_name);

        // Undo
        state = update_state(state, Some(Input::Character('<')), 30, 80);
        assert!(!state.unstaged_screen.untracked_files.is_empty());
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
        assert!(!state.unstaged_screen.unstaged_files.is_empty());

        // Navigate to the modified file
        state.unstaged_screen.unstaged_cursor = 1;

        // Press Enter to stage the file
        handle_input(&mut state, Input::Character('\n'), 30);

        // Check that the file is staged
        state.refresh_diff();
        assert!(state.unstaged_screen.unstaged_files.is_empty());
        assert_eq!(state.files[0].file_name, file_name);

        // Undo
        let state = update_state(state, Some(Input::Character('<')), 30, 80);
        assert!(!state.unstaged_screen.unstaged_files.is_empty());
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
        state.unstaged_screen.unstaged_cursor = 1;
        let hunk_line = state.unstaged_screen.unstaged_files[0]
            .lines
            .iter()
            .position(|l| l.contains("MODIFIED"))
            .unwrap();
        state.main_screen.line_cursor = hunk_line;

        // Press Enter to stage the hunk
        handle_input(&mut state, Input::Character('\n'), 30);
        state.refresh_diff();

        // Check that the hunk is staged
        assert!(state.unstaged_screen.unstaged_files.is_empty());
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
        state.unstaged_screen.unstaged_cursor = 1;
        let added_line_index = state.unstaged_screen.unstaged_files[0]
            .lines
            .iter()
            .position(|l| l.contains("ADDED"))
            .unwrap();
        state.main_screen.line_cursor = added_line_index;

        // Press '1' to stage the line
        handle_input(&mut state, Input::Character('1'), 30);
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