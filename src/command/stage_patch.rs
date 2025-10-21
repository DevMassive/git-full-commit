use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct StagePatchCommand {
    pub repo_path: PathBuf,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StagePatchCommand {
    pub fn new(repo_path: PathBuf, patch: String) -> Self {
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StagePatchCommand {
    fn execute(&mut self) -> bool {
        git::apply_patch(&self.repo_path, &self.patch, false, true)
            .expect("Failed to apply patch.");
        true
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to apply patch in reverse.");
    }

    command_impl!();
}
