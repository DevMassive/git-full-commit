use crate::git_test::common::TestRepo;
use git_full_commit::app_state::{AppState, Screen};
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

#[test]
fn test_stage_entire_file_from_unstaged_screen() {
    // Setup repo with an unstaged file
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");
    repo.add_all();
    repo.commit("initial");
    repo.create_file("a.txt", "world");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert!(app_state.files.is_empty(), "Should have no staged files initially");

    // Switch to unstaged screen
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.screen, Screen::Unstaged);

    // In the unstaged screen, list is [Header, File], so cursor on file is 1
    app_state.unstaged_screen.unstaged_cursor = 1;

    // Press 'u' to stage the file
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);

    // The file should be removed from the unstaged list
    assert!(app_state.unstaged_screen.unstaged_files.is_empty());

    // The file should now be in the staged list
    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.files[0].file_name, "a.txt");
}

#[test]
fn test_stage_untracked_file_from_unstaged_screen() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "hello");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert!(app_state.files.is_empty());
    assert_eq!(app_state.unstaged_screen.untracked_files.len(), 1);

    // Switch to unstaged screen
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    assert_eq!(app_state.screen, Screen::Unstaged);

    // list is [Unstaged H, Untracked H, File], so cursor on file is 2
    app_state.unstaged_screen.unstaged_cursor = 2;

    // Press 'u' to stage the file
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);

    assert!(app_state.unstaged_screen.untracked_files.is_empty());
    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.files[0].file_name, "a.txt");
}

#[test]
fn test_stage_hunk_from_unstaged_screen() {
    let repo = TestRepo::new();
    let initial_content: String = (1..=10).map(|i| format!("line{}\n", i)).collect();
    repo.create_file("a.txt", &initial_content);
    repo.add_all();
    repo.commit("initial");

    let mut lines: Vec<String> = initial_content.lines().map(String::from).collect();
    lines[0] = "changed1".to_string();
    lines[9] = "changed10".to_string();
    repo.create_file("a.txt", &lines.join("\n"));

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);
    assert_eq!(app_state.unstaged_screen.unstaged_files.len(), 1);
    assert_eq!(app_state.unstaged_screen.unstaged_files[0].hunks.len(), 2);

    // Switch to unstaged screen and select file
    app_state = update_state(app_state, Some(Input::Character('\t')), 80, 80);
    app_state.unstaged_screen.unstaged_cursor = 1;

    // Activate diff cursor and move to the second hunk
    let line_in_second_hunk = app_state.unstaged_screen.unstaged_files[0]
        .lines
        .iter()
        .position(|l| l.contains("+changed10"))
        .unwrap();
    
    app_state.unstaged_screen.is_unstaged_diff_cursor_active = true;
    // This is tricky, the line cursor is shared. We need to set it on main_screen.
    app_state.main_screen.line_cursor = line_in_second_hunk;

    // Press 'u' to stage the hunk
    app_state = update_state(app_state, Some(Input::Character('u')), 80, 80);

    assert_eq!(app_state.unstaged_screen.unstaged_files.len(), 1);
    assert_eq!(app_state.unstaged_screen.unstaged_files[0].hunks.len(), 1);
    assert!(!app_state.unstaged_screen.unstaged_files[0].lines.iter().any(|l| l.contains("changed10")));

    assert_eq!(app_state.files.len(), 1);
    assert_eq!(app_state.files[0].hunks.len(), 1);
    assert!(app_state.files[0].lines.iter().any(|l| l.contains("changed10")));
}