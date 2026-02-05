use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

#[test]
fn test_diff_jump_from_stat_summary() {
    let repo = TestRepo::new();
    repo.create_file("file1.txt", "content1");
    repo.add_all();
    repo.commit("commit 1");

    repo.create_file("file2.txt", "content2");
    repo.add_all();
    repo.commit("commit 2");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Initial state: cursor is likely on the first item (Staged Changes Header)
    // List: [StagedHeader, Commit2, Commit1]
    // Wait, let's check the list items.
    // build_main_screen_list_items adds StagedHeader, then staged files, then Input, then commits.
    // Here: [StagedHeader, CommitMessageInput, Commit2, Commit1]

    // Move to Commit 2 (index 2)
    app_state.main_screen.file_cursor = 2;
    app_state.update_selected_commit_diff();

    // Verify we have selected commit files
    assert!(!app_state.selected_commit_files.is_empty());

    // Find a stat line and patch start in the first file's lines
    let (stat_line_index, patch_start_index) = {
        let first_file = &app_state.selected_commit_files[0];
        let stat_line_index = first_file
            .lines
            .iter()
            .position(|l| l.contains('|'))
            .expect("Should find a stat line");
        let patch_start_index = first_file
            .lines
            .iter()
            .position(|l| l.starts_with("diff --git"))
            .expect("Should find patch start");
        (stat_line_index, patch_start_index)
    };

    // Move diff cursor to the stat line
    app_state.main_screen.is_diff_cursor_active = true;
    app_state.main_screen.line_cursor = stat_line_index;

    // Press Enter to jump
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);

    // Verify we jumped to the patch start
    assert_eq!(app_state.main_screen.line_cursor, patch_start_index);
}

#[test]
fn test_diff_jump_multiple_files() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "a");
    repo.create_file("b.txt", "b");
    repo.add_all();
    repo.commit("multiple files");

    let files = git::get_diff(repo.path.clone());
    let mut app_state = AppState::new(repo.path.clone(), files);

    // Move to the commit (index 2)
    app_state.main_screen.file_cursor = 2;
    app_state.update_selected_commit_diff();

    assert_eq!(app_state.selected_commit_files.len(), 2);

    // Find stat lines and the expected offset for the second file
    let (second_stat_index, expected_offset) = {
        let first_file = &app_state.selected_commit_files[0];

        // Find stat lines. There should be at least two (one for a.txt, one for b.txt)
        let stat_indices: Vec<usize> = first_file
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains('|') && !l.starts_with("diff --git"))
            .map(|(i, _)| i)
            .collect();

        assert!(stat_indices.len() >= 2);

        // Stat for the second file should be stat_indices[1]
        let second_stat_index = stat_indices[1];
        let expected_offset = first_file.lines.len();
        (second_stat_index, expected_offset)
    };

    // Move diff cursor to the second stat line
    app_state.main_screen.is_diff_cursor_active = true;
    app_state.main_screen.line_cursor = second_stat_index;

    // Press Enter to jump
    app_state = update_state(app_state, Some(Input::Character('\n')), 80, 80);

    // Verify we jumped to the second file's diff start
    // Offset for second file is the length of the first file's lines
    assert_eq!(app_state.main_screen.line_cursor, expected_offset);
}
