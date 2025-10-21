use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct UnstageFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl UnstageFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        let patch = git::get_file_diff_patch(&repo_path, &file_name).unwrap_or_default();
        Self {
            repo_path,
            file_name,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for UnstageFileCommand {
    fn execute(&mut self) -> bool {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
        true
    }

    fn undo(&mut self) {
        if !self.patch.is_empty() {
            git::apply_patch(&self.repo_path, &self.patch, false, true)
                .expect("Failed to apply patch for unstage undo.");
        }
    }

    command_impl!();
}
