#[cfg(test)]
mod tests {
    use crate::command::{ApplyPatchCommand, Command, test_helpers::TestRepo};
    use crate::git;

    fn get_test_patch(repo: &TestRepo, file_name: &str) -> String {
        let diff = git::get_diff(repo.path.clone());
        let file_diff = diff
            .iter()
            .find(|f| f.file_name == file_name)
            .expect("File diff not found");
        file_diff.lines.join("\n") + "\n"
    }

    #[test]
    fn test_apply_patch_to_unstage() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "line1\nline2\n");
        repo.add_all();
        repo.commit("initial");

        repo.append_file(file_name, "line3\n");
        repo.add_all();

        // The patch represents adding line3
        let patch = get_test_patch(&repo, file_name);
        let mut command = ApplyPatchCommand::new(repo.path.clone(), patch);

        // Execute (apply patch in reverse to the index)
        assert_eq!(repo.get_status(), "M  test.txt\n");
        let result = command.execute();
        assert!(result);
        // The change should now be unstaged
        assert_eq!(repo.get_status(), " M test.txt\n");

        // Undo (apply patch forward to the index)
        command.undo();
        assert_eq!(repo.get_status(), "M  test.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), " M test.txt\n");
    }
}
