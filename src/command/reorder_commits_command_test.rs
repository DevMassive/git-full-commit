use crate::command::test_helpers::{commit, create_file, get_log, TestRepo};
use crate::command::{Command, ReorderCommitsCommand};

#[test]
fn test_reorder_commits_no_commits() {
    let repo = TestRepo::new();
    let mut command = ReorderCommitsCommand::new(repo.path.clone(), vec![], vec![]);
    assert!(!command.execute());
}

#[test]
fn test_reorder_commits_one_commit() {
    let repo = TestRepo::new();
    commit(&repo.path, "first");
    let log = get_log(&repo.path);
    let mut command = ReorderCommitsCommand::new(repo.path.clone(), log.clone(), log.clone());
    assert!(!command.execute());
}

#[test]
fn test_reorder_commits_same_order() {
    let repo = TestRepo::new();
    commit(&repo.path, "first");
    commit(&repo.path, "second");
    let log = get_log(&repo.path);
    let mut command = ReorderCommitsCommand::new(repo.path.clone(), log.clone(), log.clone());
    assert!(command.execute());
    let new_log = get_log(&repo.path);
    assert_eq!(log, new_log);
}

#[test]
fn test_reorder_commits_git_error() {
    let repo = TestRepo::new();
    commit(&repo.path, "first");
    create_file(&repo.path, "a.txt", "conflict");
    commit(&repo.path, "second");
    create_file(&repo.path, "a.txt", "no conflict");
    commit(&repo.path, "third");

    let mut log = get_log(&repo.path);
    log.swap(0, 1); // Swap second and third, should fail

    let mut command =
        ReorderCommitsCommand::new(repo.path.clone(), get_log(&repo.path), log.clone());
    assert!(!command.execute());
}

#[test]
fn test_reorder_commits_successfully() {
    let repo = TestRepo::new();
    commit(&repo.path, "first");
    commit(&repo.path, "second");
    commit(&repo.path, "third");

    let original_log = get_log(&repo.path);
    let mut reordered_log = original_log.clone();
    reordered_log.swap(0, 1); // Swap "third" and "second"

    let mut command =
        ReorderCommitsCommand::new(repo.path.clone(), original_log, reordered_log.clone());
    assert!(command.execute());

    let new_log = get_log(&repo.path);
    assert_eq!(
        new_log
            .iter()
            .map(|c| &c.message)
            .collect::<Vec<&String>>(),
        reordered_log
            .iter()
            .map(|c| &c.message)
            .collect::<Vec<&String>>()
    );
}

#[test]
fn test_reorder_commits_with_unstaged_changes() {
    let repo = TestRepo::new();
    commit(&repo.path, "first");
    commit(&repo.path, "second");
    create_file(&repo.path, "unstaged.txt", "unstaged content");

    let original_log = get_log(&repo.path);
    let mut reordered_log = original_log.clone();
    reordered_log.swap(0, 1); // Swap "second" and "first"

    let mut command =
        ReorderCommitsCommand::new(repo.path.clone(), original_log, reordered_log.clone());
    assert!(command.execute());

    let new_log = get_log(&repo.path);
    assert_eq!(
        new_log
            .iter()
            .map(|c| &c.message)
            .collect::<Vec<&String>>(),
        reordered_log
            .iter()
            .map(|c| &c.message)
            .collect::<Vec<&String>>()
    );

    let status = repo.get_status();
    assert!(status.contains("?? unstaged.txt"));
}
