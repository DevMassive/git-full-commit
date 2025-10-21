use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

pub struct StageUnstagedCommand {
    pub repo_path: PathBuf,
    files_to_stage: Vec<String>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageUnstagedCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let files_to_stage = git::get_unstaged_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            files_to_stage,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageUnstagedCommand {
    fn execute(&mut self) -> bool {
        for file in &self.files_to_stage {
            git::stage_file(&self.repo_path, file).expect("Failed to stage file.");
        }
        true
    }

    fn undo(&mut self) {
        for file in &self.files_to_stage {
            git::unstage_file(&self.repo_path, file).expect("Failed to unstage file.");
        }
    }

    command_impl!();
}
