#[cfg(test)]
mod tests {
    use crate::command::{Command, StageUnstagedCommand, test_helpers::TestRepo};

    #[test]
    fn test_stage_unstaged_files() {
        let repo = TestRepo::new();
        repo.create_file("file1.txt", "content1");
        repo.create_file("file2.txt", "content2");
        repo.add_all();
        repo.commit("initial commit");

        repo.append_file("file1.txt", " more");
        repo.append_file("file2.txt", " more");

        let mut command = StageUnstagedCommand::new(repo.path.clone());

        // Execute
        assert_eq!(repo.get_status(), " M file1.txt\n M file2.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "M  file1.txt\nM  file2.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), " M file1.txt\n M file2.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "M  file1.txt\nM  file2.txt\n");
    }

    #[test]
    fn test_stage_unstaged_with_untracked_files_present() {
        let repo = TestRepo::new();
        repo.create_file("modified.txt", "initial");
        repo.add_all();
        repo.commit("initial");

        repo.append_file("modified.txt", " changes");
        repo.create_file("untracked.txt", "new");

        let mut command = StageUnstagedCommand::new(repo.path.clone());

        // Execute - should only stage the modified file
        let initial_status = repo.get_status();
        assert!(initial_status.contains(" M modified.txt"));
        assert!(initial_status.contains("?? untracked.txt"));

        let result = command.execute();
        assert!(result);
        let new_status = repo.get_status();
        assert!(new_status.contains("M  modified.txt"));
        assert!(new_status.contains("?? untracked.txt"));

        // Undo
        command.undo();
        let original_status = repo.get_status();
        assert!(original_status.contains(" M modified.txt"));
        assert!(original_status.contains("?? untracked.txt"));
    }

    #[test]
    fn test_stage_unstaged_with_no_unstaged_files() {
        let repo = TestRepo::new();
        repo.create_file("untracked.txt", "new");
        repo.create_file("staged.txt", "staged");
        repo.add_file("staged.txt");

        let mut command = StageUnstagedCommand::new(repo.path.clone());

        // Execute
        let initial_status = repo.get_status();
        assert!(initial_status.contains("A  staged.txt"));
        assert!(initial_status.contains("?? untracked.txt"));

        let result = command.execute();
        assert!(result);
        // Status should not change
        assert_eq!(repo.get_status(), initial_status);

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), initial_status);
    }
}
