use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct DiscardUnstagedHunkCommand {
    pub repo_path: PathBuf,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardUnstagedHunkCommand {
    pub fn new(repo_path: PathBuf, patch: String) -> Self {
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardUnstagedHunkCommand {
    fn execute(&mut self) -> bool {
        git::apply_patch(&self.repo_path, &self.patch, true, false)
            .expect("Failed to discard hunk from working tree.");
        true
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to re-apply hunk to working tree.");
    }

    command_impl!();
}
