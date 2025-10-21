use std::fs;
use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct DiscardFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    staged_patch: String,
    is_new_file: bool,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, is_new_file: bool) -> Self {
        let staged_patch = git::get_file_diff_patch(&repo_path, &file_name).unwrap_or_default();
        Self {
            repo_path,
            file_name,
            staged_patch,
            is_new_file,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardFileCommand {
    fn execute(&mut self) -> bool {
        if git::has_unstaged_changes_in_file(&self.repo_path, &self.file_name).unwrap_or(true) {
            // Don't discard if there are unstaged changes
            return false;
        }

        if self.is_new_file {
            git::rm_cached(&self.repo_path, &self.file_name)
                .expect("Failed to remove file from index");
            fs::remove_file(self.repo_path.join(&self.file_name))
                .expect("Failed to delete new file");
        } else {
            git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
            git::checkout_file(&self.repo_path, &self.file_name).expect("Failed to checkout file.");
        }
        true
    }

    fn undo(&mut self) {
        if self.is_new_file {
            git::apply_patch(&self.repo_path, &self.staged_patch, false, true)
                .expect("Failed to re-apply patch for new file.");
            git::checkout_file(&self.repo_path, &self.file_name)
                .expect("Failed to checkout file after undoing discard.");
        } else {
            git::apply_patch(&self.repo_path, &self.staged_patch, false, false)
                .expect("Failed to re-apply patch to working tree for undo.");
            git::apply_patch(&self.repo_path, &self.staged_patch, false, true)
                .expect("Failed to re-apply patch to index for undo.");
        }
    }

    command_impl!();
}
