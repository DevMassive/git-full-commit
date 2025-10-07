#[cfg(test)]
mod tests {
    use crate::command::{Command, StageAllCommand, test_helpers::TestRepo};

    #[test]
    fn test_stage_all_modified_only() {
        let repo = TestRepo::new();
        repo.create_file("file1.txt", "content1");
        repo.create_file("file2.txt", "content2");
        repo.add_all();
        repo.commit("initial commit");

        repo.append_file("file1.txt", " more");
        repo.append_file("file2.txt", " more");

        let mut command = StageAllCommand::new(repo.path.clone());

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
    fn test_stage_all_untracked_only() {
        let repo = TestRepo::new();
        repo.create_file("file1.txt", "content1");
        repo.create_file("file2.txt", "content2");

        let mut command = StageAllCommand::new(repo.path.clone());

        // Execute
        assert_eq!(repo.get_status(), "?? file1.txt\n?? file2.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "A  file1.txt\nA  file2.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "?? file1.txt\n?? file2.txt\n");
    }

    #[test]
    fn test_stage_all_mixed_statuses() {
        let repo = TestRepo::new();
        repo.create_file("modified.txt", "initial");
        repo.add_all();
        repo.commit("initial");

        repo.append_file("modified.txt", " changes");
        repo.create_file("untracked.txt", "new");

        let mut command = StageAllCommand::new(repo.path.clone());

        // Execute
        let initial_status = repo.get_status();
        assert!(initial_status.contains(" M modified.txt"));
        assert!(initial_status.contains("?? untracked.txt"));

        let result = command.execute();
        assert!(result);
        let staged_status = repo.get_status();
        assert!(staged_status.contains("M  modified.txt"));
        assert!(staged_status.contains("A  untracked.txt"));

        // Undo
        command.undo();
        let unstaged_status = repo.get_status();
        assert!(unstaged_status.contains(" M modified.txt"));
        assert!(unstaged_status.contains("?? untracked.txt"));
    }

    #[test]
    fn test_stage_all_with_no_changes() {
        let repo = TestRepo::new();
        repo.create_file("file.txt", "content");
        repo.add_all();
        repo.commit("initial");

        let mut command = StageAllCommand::new(repo.path.clone());

        // Execute
        assert_eq!(repo.get_status(), "");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "");
    }
}
