use std::fs;
use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;

pub struct DeleteUntrackedFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    content: Vec<u8>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DeleteUntrackedFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, content: Vec<u8>) -> Self {
        Self {
            repo_path,
            file_name,
            content,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DeleteUntrackedFileCommand {
    fn execute(&mut self) -> bool {
        fs::remove_file(self.repo_path.join(&self.file_name)).expect("Failed to delete file");
        true
    }

    fn undo(&mut self) {
        fs::write(self.repo_path.join(&self.file_name), &self.content)
            .expect("Failed to restore file");
    }

    command_impl!();
}
