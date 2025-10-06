use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

#[test]
fn test_unstage_entire_file_from_main_screen() {
    // Setup repo with a staged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Cursor starts on the file at index 1
    assert_eq!(app_state.main_screen.file_cursor, 1);
    assert!(!app_state.main_screen.is_diff_cursor_active);
    assert_eq!(app_state.files.len(), 1);

    // Press 'u' to unstage the file
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);

    // The file should be removed from the staged list
    assert_eq!(app_state.files.len(), 0, "File should be unstaged");

    // A new file that is unstaged becomes untracked
    let untracked_files = git::get_untracked_files(&repo.path).unwrap();
    assert_eq!(untracked_files.len(), 1);
    assert_eq!(untracked_files[0], "a.txt");
}

#[test]
fn test_unstage_hunk_from_main_screen() {
    // Setup repo with a file with two hunks
    let repo = TestRepo::new();
    let mut initial_content = String::new();
    for i in 1..=10 {
        initial_content.push_str(&format!("line{}\n", i));
    }
    repo.create_file("a.txt", &initial_content);
    repo.add_all();
    repo.commit("initial");

    let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
    lines[0] = "changed1".to_string();
    lines[9] = "changed10".to_string();
    repo.create_file("a.txt", &lines.join("\n"));
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.files[0].hunks.len(), 2, "Should have 2 hunks initially");

    // Navigate to the file
    app_state.main_screen.file_cursor = 1;

    // Activate diff cursor and move to the second hunk
    let line_in_second_hunk = app_state.files[0]
        .lines
        .iter()
        .position(|l| l.contains("+changed10"))
        .unwrap();
    
    app_state.main_screen.line_cursor = line_in_second_hunk;
    app_state.main_screen.is_diff_cursor_active = true;

    // Press 'u' to unstage the hunk
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);

    // The file should still be staged, but with only one hunk
    assert_eq!(app_state.files.len(), 1, "File should still be staged");
    assert_eq!(app_state.files[0].hunks.len(), 1, "Should have 1 hunk remaining");
    assert!(!app_state.files[0].lines.iter().any(|l| l.contains("changed10")));

    // The unstaged changes should now contain the second hunk
    let unstaged_files = git::get_unstaged_diff(&repo.path);
    assert_eq!(unstaged_files.len(), 1);
    assert_eq!(unstaged_files[0].hunks.len(), 1);
    assert!(unstaged_files[0].lines.iter().any(|l| l.contains("changed10")));
}