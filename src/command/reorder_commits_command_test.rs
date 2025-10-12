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

#[test]
fn test_reorder_commits_with_fixup() {
    let repo = TestRepo::new();
    repo.create_file("a.txt", "a");
    repo.add_all();
    repo.commit("first");

    repo.create_file("a.txt", "ab");
    repo.add_all();
    repo.commit("second");

    repo.create_file("b.txt", "b");
    repo.add_all();
    repo.commit("third");

    let original_log = get_log(&repo.path);
    let mut reordered_log = original_log.clone();

    // Mark "second" (index 1) as a fixup of "first" (index 2)
    if let Some(commit_to_fixup) = reordered_log.get_mut(1) {
        commit_to_fixup.is_fixup = true;
    }

    let mut command =
        ReorderCommitsCommand::new(repo.path.clone(), original_log, reordered_log);
    assert!(command.execute());

    let new_log = get_log(&repo.path);
    assert_eq!(new_log.len(), 2);
    assert_eq!(new_log[0].message, "third");
    assert_eq!(new_log[1].message, "first");

    // Verify the content of the squashed commit
    let a_txt_content = repo.get_file_content_at_commit("a.txt", &new_log[1].hash);
    assert_eq!(a_txt_content, "ab");

    // Verify the content of the third commit
    let b_txt_content = repo.get_file_content_at_commit("b.txt", &new_log[0].hash);
    assert_eq!(b_txt_content, "b");
}

#[test]
fn test_reorder_commits_with_message_change() {
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");

    let original_commits = get_log(&repo.path);
    let mut reordered_commits = original_commits.clone();

    // Swap commit 2 and 1
    reordered_commits.swap(0, 1);

    // find "commit 1" and change it's message to "new message"
    reordered_commits
        .iter_mut()
        .find(|c| c.message == "commit 1")
        .unwrap()
        .message = "new message".to_string();

    let mut command = ReorderCommitsCommand::new(
        repo.path.clone(),
        original_commits.clone(),
        reordered_commits.clone(),
    );

    let result = command.execute();
    assert!(result);

    let log = get_log(&repo.path);
    assert_eq!(
        log.iter().map(|c| &c.message).collect::<Vec<_>>(),
        vec!["new message", "commit 2", "commit 0"]
    );
}
