use crate::command::{Command, CommandHistory};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::git::{
    self, FileDiff, get_diff, get_previous_commit_diff, get_previous_commit_info,
    get_unstaged_diff, get_untracked_files, is_commit_on_remote,
};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Screen {
    Main,
    Unstaged,
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub scroll: usize,
    pub file_list_scroll: usize,
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
    pub previous_commit_is_on_remote: bool,
    pub previous_commit_files: Vec<FileDiff>,
    pub is_diff_cursor_active: bool,
    pub has_unstaged_changes: bool,
    pub screen: Screen,
    pub unstaged_files: Vec<FileDiff>,
    pub untracked_files: Vec<String>,
    pub unstaged_cursor: usize,
    pub unstaged_scroll: usize,
    pub unstaged_diff_scroll: usize,
    pub unstaged_horizontal_scroll: usize,
}

impl AppState {
    pub fn new(repo_path: PathBuf, files: Vec<FileDiff>) -> Self {
        let commit_message =
            commit_storage::load_commit_message(&repo_path).unwrap_or_else(|_| String::new());
        let (previous_commit_hash, previous_commit_message) =
            get_previous_commit_info(&repo_path).unwrap_or((String::new(), String::new()));
        let previous_commit_is_on_remote =
            is_commit_on_remote(&repo_path, &previous_commit_hash).unwrap_or(false);
        let previous_commit_files =
            get_previous_commit_diff(&repo_path).unwrap_or_else(|_| Vec::new());
        let has_unstaged_changes = git::has_unstaged_changes(&repo_path).unwrap_or(false);
        let unstaged_files = get_unstaged_diff(&repo_path);
        let untracked_files = get_untracked_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            scroll: 0,
            file_list_scroll: 0,
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
            previous_commit_is_on_remote,
            previous_commit_files,
            is_diff_cursor_active: false,
            has_unstaged_changes,
            screen: Screen::Main,
            unstaged_files,
            untracked_files,
            unstaged_cursor: 0,
            unstaged_scroll: 0,
            unstaged_diff_scroll: 0,
            unstaged_horizontal_scroll: 0,
        }
    }

    pub fn get_cursor_line_index(&self) -> usize {
        let num_files = self.files.len();
        if self.file_cursor == 0
            || (self.file_cursor > 0 && self.file_cursor <= num_files)
            || self.file_cursor == num_files + 2
        {
            self.line_cursor
        } else {
            0
        }
    }

    pub fn refresh_diff(&mut self) {
        self.files = get_diff(self.repo_path.clone());
        (self.previous_commit_hash, self.previous_commit_message) =
            get_previous_commit_info(&self.repo_path).unwrap_or((String::new(), String::new()));
        self.previous_commit_is_on_remote =
            is_commit_on_remote(&self.repo_path, &self.previous_commit_hash).unwrap_or(false);
        self.previous_commit_files =
            get_previous_commit_diff(&self.repo_path).unwrap_or_else(|_| Vec::new());
        self.has_unstaged_changes = git::has_unstaged_changes(&self.repo_path).unwrap_or(false);
        self.unstaged_files = get_unstaged_diff(&self.repo_path);
        self.untracked_files = get_untracked_files(&self.repo_path).unwrap_or_default();

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

    pub fn execute_and_refresh(&mut self, command: Box<dyn Command>) {
        let cursor_state = CursorState::from_app_state(self);
        self.command_history.execute(command, cursor_state);
        self.refresh_diff();
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        if self.file_cursor > 0 && self.file_cursor <= self.files.len() {
            self.files.get(self.file_cursor - 1)
        } else {
            None
        }
    }

    pub fn get_unstaged_file(&self) -> Option<&FileDiff> {
        if self.unstaged_cursor > 0 && self.unstaged_cursor <= self.unstaged_files.len() {
            self.unstaged_files.get(self.unstaged_cursor - 1)
        } else {
            None
        }
    }

    pub fn main_header_height(&self, max_y: i32) -> (usize, usize) {
        let num_files = self.files.len();
        let file_list_total_items = num_files + 3;
        let height = (max_y as usize / 3).max(3).min(file_list_total_items);
        (height, file_list_total_items)
    }

    pub fn unstaged_header_height(&self, max_y: i32) -> (usize, usize) {
        let unstaged_file_count = self.unstaged_files.len();
        let untracked_file_count = self.untracked_files.len();
        let file_list_total_items = unstaged_file_count + untracked_file_count + 2;
        let height = (max_y as usize / 3).max(3).min(file_list_total_items);
        (height, file_list_total_items)
    }
}
