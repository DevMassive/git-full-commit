#[cfg(test)]
mod tests {
    use crate::command::{Command, StageUntrackedCommand, test_helpers::TestRepo};

    #[test]
    fn test_stage_untracked_files() {
        let repo = TestRepo::new();
        repo.create_file("file1.txt", "content1");
        repo.create_file("file2.txt", "content2");

        let mut command = StageUntrackedCommand::new(repo.path.clone());

        // Execute
        assert_eq!(repo.get_status(), "?? file1.txt\n?? file2.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "A  file1.txt\nA  file2.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "?? file1.txt\n?? file2.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "A  file1.txt\nA  file2.txt\n");
    }

    #[test]
    fn test_stage_untracked_with_modified_files_present() {
        let repo = TestRepo::new();
        repo.create_file("modified.txt", "initial");
        repo.add_all();
        repo.commit("initial");

        repo.append_file("modified.txt", " changes");
        repo.create_file("untracked.txt", "new");

        let mut command = StageUntrackedCommand::new(repo.path.clone());

        // Execute - should only stage the untracked file
        let initial_status = repo.get_status();
        assert!(initial_status.contains(" M modified.txt"));
        assert!(initial_status.contains("?? untracked.txt"));

        let result = command.execute();
        assert!(result);
        let new_status = repo.get_status();
        assert!(new_status.contains(" M modified.txt"));
        assert!(new_status.contains("A  untracked.txt"));

        // Undo
        command.undo();
        let original_status = repo.get_status();
        assert!(original_status.contains(" M modified.txt"));
        assert!(original_status.contains("?? untracked.txt"));
    }

    #[test]
    fn test_stage_untracked_with_no_untracked_files() {
        let repo = TestRepo::new();
        repo.create_file("modified.txt", "initial");
        repo.add_all();
        repo.commit("initial");
        repo.append_file("modified.txt", " changes");

        let mut command = StageUntrackedCommand::new(repo.path.clone());

        // Execute
        let initial_status = repo.get_status();
        assert_eq!(initial_status, " M modified.txt\n");

        let result = command.execute();
        assert!(result);
        // Status should not change
        assert_eq!(repo.get_status(), initial_status);

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), initial_status);
    }
}
