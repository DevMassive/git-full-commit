use git_full_commit::git::{self, apply_patch, get_diff};
use serial_test::serial;
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

#[test]
fn test_get_local_commits() {
    // 1. Setup a repo and a remote
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    let remote_dir = TempDir::new().unwrap();
    let remote_path = remote_dir.path().to_path_buf();
    run_git(&remote_path, &["init", "--bare"]);

    run_git(
        &repo_path,
        &["remote", "add", "origin", remote_path.to_str().unwrap()],
    );
    run_git(&repo_path, &["checkout", "-b", "master"]);

    // 2. Create and push 2 commits
    fs::write(repo_path.join("a.txt"), "a").unwrap();
    run_git(&repo_path, &["add", "a.txt"]);
    run_git(&repo_path, &["commit", "-m", "commit 1"]);

    fs::write(repo_path.join("b.txt"), "b").unwrap();
    run_git(&repo_path, &["add", "b.txt"]);
    run_git(&repo_path, &["commit", "-m", "commit 2"]);
    run_git(&repo_path, &["push", "origin", "master"]);

    // 3. Create 2 local commits
    fs::write(repo_path.join("c.txt"), "c").unwrap();
    run_git(&repo_path, &["add", "c.txt"]);
    run_git(&repo_path, &["commit", "-m", "commit 3"]);

    fs::write(repo_path.join("d.txt"), "d").unwrap();
    run_git(&repo_path, &["add", "d.txt"]);
    run_git(&repo_path, &["commit", "-m", "commit 4"]);

    // 4. Call get_local_commits
    let commits = git::get_local_commits(&repo_path).unwrap();

    // 5. Assert results
    assert_eq!(commits.len(), 3);
    assert_eq!(commits[0].message, "commit 4");
    assert!(!commits[0].is_on_remote);
    assert_eq!(commits[1].message, "commit 3");
    assert!(!commits[1].is_on_remote);
    assert_eq!(commits[2].message, "commit 2");
    assert!(commits[2].is_on_remote);
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

fn get_commit_messages(repo_path: &PathBuf, count: usize) -> Vec<String> {
    let output = OsCommand::new("git")
        .arg("log")
        .arg(format!("-n{count}"))
        .arg("--pretty=%s")
        .current_dir(repo_path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

fn get_commit_hash(repo_path: &PathBuf, rev: &str) -> String {
    let output = OsCommand::new("git")
        .arg("rev-parse")
        .arg(rev)
        .current_dir(repo_path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_reword_commit_on_non_head_updates_only_target() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    fs::write(repo_path.join("a.txt"), "one").unwrap();
    run_git(&repo_path, &["add", "a.txt"]);
    run_git(&repo_path, &["commit", "-m", "first commit"]);

    fs::write(repo_path.join("b.txt"), "two").unwrap();
    run_git(&repo_path, &["add", "b.txt"]);
    run_git(&repo_path, &["commit", "-m", "second commit"]);

    let target_hash = get_commit_hash(&repo_path, "HEAD~1");
    git::reword_commit(&repo_path, &target_hash, "rewritten first commit").unwrap();

    let messages = get_commit_messages(&repo_path, 2);
    assert_eq!(messages[0], "second commit");
    assert_eq!(messages[1], "rewritten first commit");
}

#[test]
fn test_amend_commit_with_staged_changes_on_non_head_updates_only_target() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path().to_path_buf();
    run_git(&repo_path, &["init"]);
    run_git(&repo_path, &["config", "user.name", "Test"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);

    fs::write(repo_path.join("a.txt"), "one").unwrap();
    run_git(&repo_path, &["add", "a.txt"]);
    run_git(&repo_path, &["commit", "-m", "first commit"]);

    fs::write(repo_path.join("b.txt"), "two").unwrap();
    run_git(&repo_path, &["add", "b.txt"]);
    run_git(&repo_path, &["commit", "-m", "second commit"]);

    fs::write(repo_path.join("c.txt"), "staged file").unwrap();
    run_git(&repo_path, &["add", "c.txt"]);

    let target_hash = get_commit_hash(&repo_path, "HEAD~1");
    git::amend_commit_with_staged_changes(
        &repo_path,
        &target_hash,
        "first commit with staged changes",
    )
    .unwrap();

    let messages = get_commit_messages(&repo_path, 2);
    assert_eq!(messages[0], "second commit");
    assert_eq!(messages[1], "first commit with staged changes");

    let amended_diff = OsCommand::new("git")
        .arg("show")
        .arg("HEAD~1")
        .current_dir(&repo_path)
        .output()
        .unwrap();
    let diff_str = String::from_utf8_lossy(&amended_diff.stdout);
    assert!(
        diff_str.contains("c.txt"),
        "Expected staged file to be part of amended commit diff. Diff: {}",
        diff_str
    );
}

#[test]
fn test_get_commit_diff() {
    let setup = TestSetup::new();

    // Commit the staged changes to create a new commit
    run_git(&setup.repo_path, &["commit", "-m", "second commit"]);

    // Call the function to get the diff of the last commit
    let diffs = git_full_commit::git::get_commit_diff(&setup.repo_path, "HEAD").unwrap();

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
