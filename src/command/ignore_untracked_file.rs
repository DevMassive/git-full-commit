use std::fs;
use std::io::Write;
use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct IgnoreUntrackedFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    was_empty_before: bool,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl IgnoreUntrackedFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        let gitignore_path = repo_path.join(".gitignore");
        let was_empty_before = !gitignore_path.exists()
            || fs::read_to_string(gitignore_path)
                .map(|c| c.trim().is_empty())
                .unwrap_or(true);
        Self {
            repo_path,
            file_name,
            was_empty_before,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for IgnoreUntrackedFileCommand {
    fn execute(&mut self) -> bool {
        let gitignore_path = self.repo_path.join(".gitignore");
        let mut gitignore = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore, "{}", self.file_name).expect("Failed to write to .gitignore");

        git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
        true
    }

    fn undo(&mut self) {
        let gitignore_path = self.repo_path.join(".gitignore");
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
            let new_content: String = content
                .lines()
                .filter(|line| !line.trim().is_empty() && *line != self.file_name)
                .collect::<Vec<_>>()
                .join("\n");

            if new_content.is_empty() {
                fs::remove_file(&gitignore_path).expect("Failed to remove .gitignore");
                if !self.was_empty_before {
                    // If the file was not empty before, we need to remove it from the index
                    git::rm_cached(&self.repo_path, ".gitignore")
                        .expect("Failed to remove .gitignore from index");
                }
            } else {
                fs::write(&gitignore_path, new_content + "\n")
                    .expect("Failed to write to .gitignore");
                git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
            }
        }
    }

    command_impl!();
}
