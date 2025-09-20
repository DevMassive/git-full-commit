use git_full_commit::git::{FileDiff, FileStatus, Hunk};
use git_full_commit::git_patch;

fn create_test_file_diff() -> FileDiff {
    let lines = vec![
        "@@ -1,5 +1,6 @@".to_string(),
        " line 1".to_string(),
        "-line 2".to_string(),
        "-line 3".to_string(),
        "+line 2 new".to_string(),
        "+line 3 new".to_string(),
        " line 4".to_string(),
    ];
    // This is a simplified version of what `calc_line_numbers` produces.
    // The main thing is that the `new_line_num` is correct.
    let line_numbers = vec![
        (0, 0), // @@
        (1, 1), // ` line 1`
        (2, 1), // `-line 2` -> new line number is 1 (the line before)
        (3, 1), // `-line 3` -> new line number is 1
        (3, 2), // `+line 2 new` -> new line number is 2
        (3, 3), // `+line 3 new` -> new line number is 3
        (4, 4), // ` line 4`
    ];
    let hunks = vec![Hunk {
        start_line: 0,
        lines: lines.clone(),
        old_start: 1,
        new_start: 1,
        line_numbers,
    }];
    FileDiff {
        file_name: "test.txt".to_string(),
        hunks,
        lines,
        status: FileStatus::Modified,
    }
}

#[test]
fn test_get_line_number() {
    let file = create_test_file_diff();

    // Hunk header -> None
    assert_eq!(git_patch::get_line_number(&file, 0), None);
    // Context line -> Some(1)
    assert_eq!(git_patch::get_line_number(&file, 1), Some(1));
    // Deleted line -> Some(2) (line number after the previous valid line)
    assert_eq!(git_patch::get_line_number(&file, 2), Some(2));
    assert_eq!(git_patch::get_line_number(&file, 3), Some(2));
    // Added line -> Some(2)
    assert_eq!(git_patch::get_line_number(&file, 4), Some(2));
    assert_eq!(git_patch::get_line_number(&file, 5), Some(3));
    // Context line -> Some(4)
    assert_eq!(git_patch::get_line_number(&file, 6), Some(4));
    // Out of bounds -> None
    assert_eq!(git_patch::get_line_number(&file, 7), None);
}
