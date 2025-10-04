#[cfg(test)]
mod tests {
    use crate::command::{test_helpers::TestRepo, Command, StageFileCommand};

    #[test]
    fn test_stage_modified_file() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "content");
        repo.add_file(file_name);
        repo.commit("initial commit");
        repo.append_file(file_name, " more content");

        let mut command = StageFileCommand::new(repo.path.clone(), file_name.to_string());

        // Execute
        assert_eq!(repo.get_status(), " M test.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "M  test.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), " M test.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "M  test.txt\n");
    }

    #[test]
    fn test_stage_untracked_file() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "content");

        let mut command = StageFileCommand::new(repo.path.clone(), file_name.to_string());

        // Execute
        assert_eq!(repo.get_status(), "?? test.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "A  test.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "?? test.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "A  test.txt\n");
    }

    #[test]
    fn test_stage_already_staged_file() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "content");
        repo.add_file(file_name);

        let mut command = StageFileCommand::new(repo.path.clone(), file_name.to_string());

        // Execute
        assert_eq!(repo.get_status(), "A  test.txt\n");
        let result = command.execute();
        assert!(result);
        // Status should not change
        assert_eq!(repo.get_status(), "A  test.txt\n");

        // Undo
        command.undo();
        // The undo operation will unstage the file, making it untracked.
        assert_eq!(repo.get_status(), "?? test.txt\n");
    }
}