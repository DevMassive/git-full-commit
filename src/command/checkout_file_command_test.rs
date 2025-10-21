#[cfg(test)]
mod tests {
    use crate::command::{CheckoutFileCommand, Command, test_helpers::TestRepo};
    use crate::git;

    #[test]
    fn test_checkout_file() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "line1\nline2\n");
        repo.add_all();
        repo.commit("initial");

        repo.append_file(file_name, "line3\n"); // Unstaged change

        // The command needs the patch to be able to undo
        let patch = git::get_unstaged_file_diff_patch(&repo.path, file_name).unwrap();
        let mut command = CheckoutFileCommand::new(repo.path.clone(), file_name.to_string(), patch);

        // Execute
        assert_eq!(repo.get_status(), " M test.txt\n");
        let result = command.execute();
        assert!(result);
        // The unstaged changes should be gone
        assert_eq!(repo.get_status(), "");

        // Undo
        command.undo();
        assert_eq!(repo.get_status(), " M test.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "");
    }
}
