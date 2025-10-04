#[cfg(test)]
mod tests {
    use crate::command::{test_helpers::TestRepo, Command, StagePatchCommand};
    use crate::git;

    fn get_unstaged_test_patch(repo: &TestRepo, file_name: &str) -> String {
        let diff = git::get_unstaged_diff(&repo.path);
        let file_diff = diff
            .iter()
            .find(|f| f.file_name == file_name)
            .expect("File diff not found");
        file_diff.lines.join("\n") + "\n"
    }

    #[test]
    fn test_stage_patch() {
        let repo = TestRepo::new();
        let file_name = "test.txt";
        repo.create_file(file_name, "line1\nline2\n");
        repo.add_all();
        repo.commit("initial");

        repo.append_file(file_name, "line3\n"); // Unstaged change

        let patch = get_unstaged_test_patch(&repo, file_name);
        let mut command = StagePatchCommand::new(repo.path.clone(), patch);

        // Execute (apply patch forward to the index)
        assert_eq!(repo.get_status(), " M test.txt\n");
        let result = command.execute();
        assert!(result);
        // The change should now be staged
        assert_eq!(repo.get_status(), "M  test.txt\n");

        // Undo (apply patch in reverse to the index)
        command.undo();
        assert_eq!(repo.get_status(), " M test.txt\n");

        // Redo
        let result = command.execute();
        assert!(result);
        assert_eq!(repo.get_status(), "M  test.txt\n");
    }
}