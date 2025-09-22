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

pub struct EditorRequest {
    pub file_path: String,
    pub line_number: Option<usize>,
}

pub struct MainScreenState {
    pub diff_scroll: usize,
    pub file_list_scroll: usize,
    pub horizontal_scroll: usize,
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub commit_message: String,
    pub is_commit_mode: bool,
    pub commit_cursor: usize,
    pub amend_message: String,
    pub is_amend_mode: bool,
    pub is_diff_cursor_active: bool,
    pub has_unstaged_changes: bool,
}
impl Default for MainScreenState {
    fn default() -> Self {
        Self {
            diff_scroll: 0,
            file_list_scroll: 0,
            horizontal_scroll: 0,
            file_cursor: 0,
            line_cursor: 0,
            commit_message: String::new(),
            is_commit_mode: false,
            commit_cursor: 0,
            amend_message: String::new(),
            is_amend_mode: false,
            is_diff_cursor_active: false,
            has_unstaged_changes: false,
        }
    }
}

pub struct UnstagedScreenState {
    pub unstaged_files: Vec<FileDiff>,
    pub untracked_files: Vec<String>,
    pub unstaged_cursor: usize,
    pub unstaged_scroll: usize,
    pub unstaged_diff_scroll: usize,
    pub unstaged_horizontal_scroll: usize,
    pub is_unstaged_diff_cursor_active: bool,
}
impl Default for UnstagedScreenState {
    fn default() -> Self {
        Self {
            unstaged_files: Vec::new(),
            untracked_files: Vec::new(),
            unstaged_cursor: 0,
            unstaged_scroll: 0,
            unstaged_diff_scroll: 0,
            unstaged_horizontal_scroll: 0,
            is_unstaged_diff_cursor_active: false,
        }
    }
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub main_screen: MainScreenState,
    pub unstaged_screen: UnstagedScreenState,
    pub running: bool,
    pub files: Vec<FileDiff>,
    pub command_history: CommandHistory,
    pub previous_commit_hash: String,
    pub previous_commit_message: String,
    pub previous_commit_is_on_remote: bool,
    pub previous_commit_files: Vec<FileDiff>,
    pub screen: Screen,
    pub editor_request: Option<EditorRequest>,
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
        let mut main_screen = MainScreenState::default();
        main_screen.commit_message = commit_message;
        main_screen.file_cursor = if files.len() > 0 { 1 } else { 0 };
        main_screen.has_unstaged_changes = has_unstaged_changes;
        let mut unstaged_screen = UnstagedScreenState::default();
        unstaged_screen.unstaged_files = unstaged_files;
        unstaged_screen.untracked_files = untracked_files;
        Self {
            repo_path,
            main_screen,
            unstaged_screen,
            running: true,
            files,
            command_history: CommandHistory::new(),
            previous_commit_hash,
            previous_commit_message,
            previous_commit_is_on_remote,
            previous_commit_files,
            screen: Screen::Main,
            editor_request: None,
        }
    }

    pub fn get_cursor_line_index(&self) -> usize {
        let num_files = self.files.len();
        if self.main_screen.file_cursor == 0
            || (self.main_screen.file_cursor > 0 && self.main_screen.file_cursor <= num_files)
            || self.main_screen.file_cursor == num_files + 2
        {
            self.main_screen.line_cursor
        } else {
            0
        }
    }

    pub fn refresh_diff(&mut self) {
        let old_file_cursor = self.main_screen.file_cursor;
        let old_line_cursor = self.main_screen.line_cursor;
        let old_scroll = self.main_screen.diff_scroll;
        let old_file_list_scroll = self.main_screen.file_list_scroll;
        let old_unstaged_cursor = self.unstaged_screen.unstaged_cursor;
        let old_unstaged_scroll = self.unstaged_screen.unstaged_scroll;
        let old_unstaged_diff_scroll = self.unstaged_screen.unstaged_diff_scroll;

        self.files = get_diff(self.repo_path.clone());
        (self.previous_commit_hash, self.previous_commit_message) =
            get_previous_commit_info(&self.repo_path).unwrap_or((String::new(), String::new()));
        self.previous_commit_is_on_remote =
            is_commit_on_remote(&self.repo_path, &self.previous_commit_hash).unwrap_or(false);
        self.previous_commit_files =
            get_previous_commit_diff(&self.repo_path).unwrap_or_else(|_| Vec::new());
        self.main_screen.has_unstaged_changes =
            git::has_unstaged_changes(&self.repo_path).unwrap_or(false);
        self.unstaged_screen.unstaged_files = get_unstaged_diff(&self.repo_path);
        self.unstaged_screen.untracked_files = get_untracked_files(&self.repo_path).unwrap_or_default();

        if self.files.is_empty() {
            self.main_screen.file_cursor = 0;
            self.main_screen.line_cursor = 0;
            self.main_screen.diff_scroll = 0;
        } else {
            self.main_screen.file_cursor = old_file_cursor.min(self.files.len() + 1);
            if let Some(file) = self.current_file() {
                let max_line = file.lines.len().saturating_sub(1);
                self.main_screen.line_cursor = old_line_cursor.min(max_line);
                self.main_screen.diff_scroll = old_scroll.min(max_line);
            } else {
                self.main_screen.line_cursor = 0;
                self.main_screen.diff_scroll = 0;
            }
        }
        self.main_screen.file_list_scroll = old_file_list_scroll;

        let unstaged_file_count = self.unstaged_screen.unstaged_files.len();
        let untracked_file_count = self.unstaged_screen.untracked_files.len();
        let max_unstaged_cursor = unstaged_file_count + untracked_file_count + 1;
        self.unstaged_screen.unstaged_cursor = old_unstaged_cursor.min(max_unstaged_cursor);
        self.unstaged_screen.unstaged_scroll = old_unstaged_scroll;
        self.unstaged_screen.unstaged_diff_scroll = old_unstaged_diff_scroll;
    }

    pub fn execute_and_refresh(&mut self, command: Box<dyn Command>) {
        let cursor_state = CursorState::from_app_state(self);
        self.command_history.execute(command, cursor_state);
        self.refresh_diff();
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        if self.main_screen.file_cursor > 0 && self.main_screen.file_cursor <= self.files.len() {
            self.files.get(self.main_screen.file_cursor - 1)
        } else {
            None
        }
    }

    pub fn get_unstaged_file(&self) -> Option<&FileDiff> {
        if self.unstaged_screen.unstaged_cursor > 0 && self.unstaged_screen.unstaged_cursor <= self.unstaged_screen.unstaged_files.len() {
            self.unstaged_screen.unstaged_files.get(self.unstaged_screen.unstaged_cursor - 1)
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
        let unstaged_file_count = self.unstaged_screen.unstaged_files.len();
        let untracked_file_count = self.unstaged_screen.untracked_files.len();
        let file_list_total_items = unstaged_file_count + untracked_file_count + 2;
        let height = (max_y as usize / 3).max(3).min(file_list_total_items);
        (height, file_list_total_items)
    }
}
