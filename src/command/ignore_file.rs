use std::fs;
use std::io::Write;
use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct IgnoreFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl IgnoreFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        Self {
            repo_path,
            file_name,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for IgnoreFileCommand {
    fn execute(&mut self) -> bool {
        let gitignore_path = self.repo_path.join(".gitignore");
        let mut gitignore = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore, "{}", self.file_name).expect("Failed to write to .gitignore");

        git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");

        // For staged files, we need to unstage them.
        git::rm_cached(&self.repo_path, &self.file_name).expect("Failed to unstage file");
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
                git::rm_file_from_index(&self.repo_path, ".gitignore")
                    .expect("Failed to remove .gitignore from index");
            } else {
                fs::write(&gitignore_path, new_content + "\n")
                    .expect("Failed to write to .gitignore");
                git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
            }
        }

        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file");
    }

    command_impl!();
}
