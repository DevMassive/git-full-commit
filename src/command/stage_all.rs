use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct StageAllCommand {
    pub repo_path: PathBuf,
    patch: String,
    untracked_files: Vec<String>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageAllCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let patch = git::get_unstaged_diff_patch(&repo_path).unwrap_or_default();
        let untracked_files = git::get_untracked_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            patch,
            untracked_files,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageAllCommand {
    fn execute(&mut self) -> bool {
        git::add_all(&self.repo_path).expect("Failed to stage all files.");
        true
    }

    fn undo(&mut self) {
        // Untracked files are now tracked, so we need to unstage them.
        for file in &self.untracked_files {
            git::rm_cached(&self.repo_path, file).expect("Failed to unstage file.");
        }

        // For modified and deleted files, we apply the reverse of the patch.
        if !self.patch.is_empty() {
            git::apply_patch(&self.repo_path, &self.patch, true, true)
                .expect("Failed to apply patch in reverse.");
        }
    }

    command_impl!();
}
