use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct UnstageAllCommand {
    pub repo_path: PathBuf,
    patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl UnstageAllCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let patch = git::get_staged_diff_patch(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for UnstageAllCommand {
    fn execute(&mut self) -> bool {
        git::unstage_all(&self.repo_path).expect("Failed to unstage all files.");
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
