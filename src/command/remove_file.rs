use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct RemoveFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl RemoveFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, patch: String) -> Self {
        Self {
            repo_path,
            file_name,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for RemoveFileCommand {
    fn execute(&mut self) -> bool {
        git::rm_file(&self.repo_path, &self.file_name).expect("Failed to remove file.");
        true
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to apply patch for remove undo.");

        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
    }

    command_impl!();
}
