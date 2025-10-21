use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct StageUntrackedCommand {
    pub repo_path: PathBuf,
    untracked_files: Vec<String>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageUntrackedCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let untracked_files = git::get_untracked_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            untracked_files,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageUntrackedCommand {
    fn execute(&mut self) -> bool {
        for file in &self.untracked_files {
            git::stage_file(&self.repo_path, file).expect("Failed to stage untracked file.");
        }
        true
    }

    fn undo(&mut self) {
        for file in &self.untracked_files {
            git::rm_cached(&self.repo_path, file).expect("Failed to unstage untracked file.");
        }
    }

    command_impl!();
}
