use super::common::TestRepo;
use git_full_commit::command::{Command, DiscardFileCommand, DiscardHunkCommand};
use git_full_commit::git;
use git_full_commit::git_patch;
use serial_test::serial;

#[test]
#[serial]
fn test_discard_file_no_conflict() {
    let repo = TestRepo::new();
    let file_name = "test.txt";
    repo.create_file(file_name, "hello\n");
    repo.add_file(file_name);
    repo.commit("initial commit");

    repo.append_file(file_name, "world\n");
    repo.add_file(file_name);

    // There are staged changes, but no unstaged changes
    let staged_diff = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff.len(), 1);
    assert!(git::get_unstaged_diff(&repo.path).is_empty());

    let mut command = DiscardFileCommand::new(repo.path.clone(), file_name.to_string(), false);
    command.execute();

    // Staged changes should be gone
    let staged_diff_after = git::get_diff(repo.path.clone());
    assert!(staged_diff_after.is_empty());

    // Working directory should be clean
    let unstaged_diff_after = git::get_unstaged_diff(&repo.path);
    assert!(unstaged_diff_after.is_empty());
}

#[test]
#[serial]
fn test_discard_file_with_conflict() {
    let repo = TestRepo::new();
    let file_name = "test.txt";
    repo.create_file(file_name, "hello\n");
    repo.add_file(file_name);
    repo.commit("initial commit");

    // Stage a change
    repo.append_file(file_name, "staged change\n");
    repo.add_file(file_name);

    // Create an unstaged change (conflict)
    repo.append_file(file_name, "unstaged change\n");

    let staged_diff_before = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff_before.len(), 1);
    assert!(!git::get_unstaged_diff(&repo.path).is_empty());

    let mut command = DiscardFileCommand::new(repo.path.clone(), file_name.to_string(), false);
    command.execute();

    // Staged changes should NOT be discarded
    let staged_diff_after = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff_after.len(), 1);
    assert_eq!(
        staged_diff_after[0].file_name,
        staged_diff_before[0].file_name
    );
}

#[test]
#[serial]
fn test_discard_new_file_no_conflict() {
    let repo = TestRepo::new();
    let file_name = "new_file.txt";
    repo.create_file(file_name, "new content\n");
    repo.add_file(file_name);

    let staged_diff_before = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff_before.len(), 1);
    assert!(staged_diff_before[0].status == git::FileStatus::Added);

    let mut command = DiscardFileCommand::new(repo.path.clone(), file_name.to_string(), true);
    command.execute();

    // Staged changes should be gone
    assert!(git::get_diff(repo.path.clone()).is_empty());

    // File should not exist
    assert!(!repo.path.join(file_name).exists());
}

#[test]
#[serial]
fn test_discard_hunk_no_conflict() {
    let repo = TestRepo::new();
    let file_name = "test.txt";
    repo.create_file(file_name, "line 1\nline 2\nline 3\n");
    repo.add_file(file_name);
    repo.commit("initial commit");

    repo.append_file(file_name, "line 4\n");
    repo.add_file(file_name);

    let staged_diff = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff.len(), 1);
    let file_diff = &staged_diff[0];
    let hunk = &file_diff.hunks[0];

    let patch = git_patch::create_unstage_hunk_patch(file_diff, hunk);
    let mut command = DiscardHunkCommand::new(repo.path.clone(), patch);
    command.execute();

    // Staged changes should be gone
    assert!(git::get_diff(repo.path.clone()).is_empty());
    // Working directory should be clean
    assert!(git::get_unstaged_diff(&repo.path).is_empty());
}

#[test]
#[serial]
fn test_discard_hunk_with_conflict() {
    let repo = TestRepo::new();
    let file_name = "test.txt";
    repo.create_file(file_name, "line 1\n");
    repo.add_file(file_name);
    repo.commit("initial commit");

    // Stage a change
    repo.append_file(file_name, "staged change\n");
    repo.add_file(file_name);

    // Create an unstaged change
    repo.append_file(file_name, "unstaged change\n");

    let staged_diff_before = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff_before.len(), 1);
    let file_diff = &staged_diff_before[0];
    let hunk = &file_diff.hunks[0];

    let patch = git_patch::create_unstage_hunk_patch(file_diff, hunk);
    let mut command = DiscardHunkCommand::new(repo.path.clone(), patch);
    command.execute();

    // Staged changes should NOT be discarded
    let staged_diff_after = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff_after.len(), 1);
}

#[test]
#[serial]
fn test_undo_discard_new_file() {
    let repo = TestRepo::new();
    let file_name = "new_file.txt";
    repo.create_file(file_name, "new content\n");
    repo.add_file(file_name);

    let mut command = DiscardFileCommand::new(repo.path.clone(), file_name.to_string(), true);
    command.execute();

    // After discard, file should be gone
    assert!(git::get_diff(repo.path.clone()).is_empty());
    assert!(!repo.path.join(file_name).exists());

    // Undo the discard
    command.undo();

    // After undo, file should be staged again
    let staged_diff = git::get_diff(repo.path.clone());
    assert_eq!(staged_diff.len(), 1);
    assert_eq!(staged_diff[0].file_name, file_name);
    assert!(staged_diff[0].status == git::FileStatus::Added);

    // And there should be no unstaged changes
    let unstaged_diff = git::get_unstaged_diff(&repo.path);
    assert!(unstaged_diff.is_empty());
}

#[test]
#[serial]
fn test_failed_discard_does_not_push_to_undo_stack() {
    let repo = TestRepo::new();
    let file_name = "test.txt";
    repo.create_file(file_name, "hello\n");
    repo.add_file(file_name);
    repo.commit("initial commit");

    // Stage a change
    repo.append_file(file_name, "staged change\n");
    repo.add_file(file_name);

    // Create an unstaged change (conflict)
    repo.append_file(file_name, "unstaged change\n");

    let mut command = DiscardFileCommand::new(repo.path.clone(), file_name.to_string(), false);
    // Execute fails due to conflict
    let executed = command.execute();
    assert!(!executed);

    // If we had a CommandHistory, we would check that the command was not pushed.
    // For this test, we'll just confirm that calling undo doesn't revert the staged change.
    command.undo();

    let staged_diff_after = git::get_diff(repo.path.clone());
    assert_eq!(
        staged_diff_after.len(),
        1,
        "Staged changes should not have been undone"
    );
}