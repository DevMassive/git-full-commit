#[cfg(test)]
mod tests {
    use crate::command::{Command, UnstageAllCommand, test_helpers::TestRepo};

    #[test]
    fn test_unstage_all_simple() {
        let repo = TestRepo::new();
        // Add an initial commit so HEAD exists
        repo.create_file("dummy.txt", "dummy");
        repo.add_all();
        repo.commit("initial commit");

        repo.create_file("file1.txt", "content1");
        repo.create_file("file2.txt", "content2");
        repo.add_all();

        let mut command = UnstageAllCommand::new(repo.path.clone());

        // Execute
        assert_eq!(repo.get_status(), "A  file1.txt\nA  file2.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "?? file1.txt\n?? file2.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "A  file1.txt\nA  file2.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "?? file1.txt\n?? file2.txt\n");
    }

    #[test]
    fn test_unstage_all_with_mixed_staged_changes() {
        let repo = TestRepo::new();
        repo.create_file("modified.txt", "initial");
        repo.create_file("added.txt", "initial");
        repo.add_all();
        repo.commit("initial");

        repo.append_file("modified.txt", " changes");
        repo.create_file("new.txt", "new");
        repo.add_all(); // Stage all changes

        let mut command = UnstageAllCommand::new(repo.path.clone());

        // Execute
        let status = repo.get_status();
        assert!(status.contains("M  modified.txt"));
        assert!(status.contains("A  new.txt"));

        let result = command.execute();
        assert!(result);

        let new_status = repo.get_status();
        assert!(new_status.contains(" M modified.txt"));
        assert!(new_status.contains("?? new.txt"));

        // Undo
        command.undo();
        let original_status = repo.get_status();
        assert!(original_status.contains("M  modified.txt"));
        assert!(original_status.contains("A  new.txt"));
    }

    #[test]
    fn test_unstage_all_with_no_staged_changes() {
        let repo = TestRepo::new();
        repo.create_file("file.txt", "content");

        let mut command = UnstageAllCommand::new(repo.path.clone());

        // Execute
        assert_eq!(repo.get_status(), "?? file.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "?? file.txt\n");

        // Undo
        command.undo();
        // Should be no change since nothing was staged to begin with.
        assert_eq!(repo.get_status(), "?? file.txt\n");
    }
}
