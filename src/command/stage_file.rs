use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct StageFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        Self {
            repo_path,
            file_name,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageFileCommand {
    fn execute(&mut self) -> bool {
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
        true
    }

    fn undo(&mut self) {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
    }

    command_impl!();
}
