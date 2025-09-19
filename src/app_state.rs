use std::path::PathBuf;
use crate::commit_storage;
use crate::git::{FileDiff, get_diff, get_previous_commit_diff, get_previous_commit_info};
use crate::command::CommandHistory;

pub struct AppState {
    pub repo_path: PathBuf,
    pub scroll: usize,
    pub horizontal_scroll: usize,
    pub running: bool,
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub files: Vec<FileDiff>,
    pub command_history: CommandHistory,
    pub commit_message: String,
    pub is_commit_mode: bool,
    pub commit_cursor: usize,
    pub amend_message: String,
    pub is_amend_mode: bool,
    pub previous_commit_hash: String,
    pub previous_commit_message: String,
    pub previous_commit_files: Vec<FileDiff>,
}

impl AppState {
    pub fn new(repo_path: PathBuf, files: Vec<FileDiff>) -> Self {
        let commit_message =
            commit_storage::load_commit_message(&repo_path).unwrap_or_else(|_| String::new());
        let (previous_commit_hash, previous_commit_message) =
            get_previous_commit_info(&repo_path).unwrap_or((String::new(), String::new()));
        let previous_commit_files = get_previous_commit_diff(&repo_path).unwrap_or_else(|_| Vec::new());
        Self {
            repo_path,
            scroll: 0,
            horizontal_scroll: 0,
            running: true,
            file_cursor: 1,
            line_cursor: 0,
            files,
            command_history: CommandHistory::new(),
            commit_message,
            is_commit_mode: false,
            commit_cursor: 0,
            amend_message: String::new(),
            is_amend_mode: false,
            previous_commit_hash,
            previous_commit_message,
            previous_commit_files,
        }
    }

    pub fn get_cursor_line_index(&self) -> usize {
        if self.file_cursor == 0 {
            return self.line_cursor;
        }
        if self.file_cursor > 0 && self.file_cursor <= self.files.len() {
            self.line_cursor
        } else {
            0
        }
    }

    pub fn refresh_diff(&mut self) {
        self.files = get_diff(self.repo_path.clone());
        (self.previous_commit_hash, self.previous_commit_message) =
            get_previous_commit_info(&self.repo_path).unwrap_or((String::new(), String::new()));
        self.previous_commit_files = get_previous_commit_diff(&self.repo_path).unwrap_or_else(|_| Vec::new());

        if self.files.is_empty() {
            self.file_cursor = 1; // commit message line
            self.line_cursor = 0;
            self.scroll = 0;
            return;
        }

        // 0: prev commit, 1..N: files, N+1: commit message
        self.file_cursor = self.file_cursor.min(self.files.len() + 1);
        self.line_cursor = 0;
        self.scroll = 0;
    }
}
