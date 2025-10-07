#[cfg(test)]
mod tests {
    use crate::command::{Command, UnstageFileCommand, test_helpers::TestRepo};

    #[test]
    fn test_unstage_staged_file() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "content");
        repo.add_file(file_name);

        let mut command = UnstageFileCommand::new(repo.path.clone(), file_name.to_string());

        // Execute
        assert_eq!(repo.get_status(), "A  test.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "?? test.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "A  test.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "?? test.txt\n");
    }

    #[test]
    fn test_unstage_file_with_unstaged_changes() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "content");
        repo.add_file(file_name);
        repo.commit("initial commit");
        repo.append_file(file_name, " more content"); // This is the change that will be staged
        repo.add_file(file_name);
        repo.append_file(file_name, " even more content"); // This change will remain unstaged

        let mut command = UnstageFileCommand::new(repo.path.clone(), file_name.to_string());

        // Execute
        assert_eq!(repo.get_status(), "MM test.txt\n");
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), " M test.txt\n");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), "MM test.txt\n");
    }

    #[test]
    fn test_unstage_file_that_is_not_staged() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "content");

        let mut command = UnstageFileCommand::new(repo.path.clone(), file_name.to_string());

        // Execute
        assert_eq!(repo.get_status(), "?? test.txt\n");
        let result = command.execute();
        assert!(result);
        // Status should not change
        assert_eq!(repo.get_status(), "?? test.txt\n");

        // Undo
        command.undo();
        // Status should still not change as the patch was empty
        assert_eq!(repo.get_status(), "?? test.txt\n");
    }
}
