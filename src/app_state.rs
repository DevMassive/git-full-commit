use crate::command::{Command, CommandHistory};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::git::{
    self, CommitInfo, FileDiff, get_commit_diff, get_diff, get_local_commits, get_unstaged_diff,
    get_untracked_files,
};
use crate::ui::main_screen::ListItem as MainScreenListItem;
use crate::ui::unstaged_screen::ListItem as UnstagedScreenListItem;
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

#[derive(Default)]
pub struct MainScreenState {
    pub diff_scroll: usize,
    pub file_list_scroll: usize,
    pub horizontal_scroll: usize,
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub commit_message: String,
    pub is_commit_mode: bool,
    pub commit_cursor: usize,
    pub amending_commit_hash: Option<String>,
    pub is_diff_cursor_active: bool,
    pub has_unstaged_changes: bool,
    pub list_items: Vec<MainScreenListItem>,
}

#[derive(Default)]
pub struct UnstagedScreenState {
    pub unstaged_files: Vec<FileDiff>,
    pub untracked_files: Vec<String>,
    pub unstaged_cursor: usize,
    pub unstaged_scroll: usize,
    pub unstaged_diff_scroll: usize,
    pub unstaged_horizontal_scroll: usize,
    pub is_unstaged_diff_cursor_active: bool,
    pub list_items: Vec<UnstagedScreenListItem>,
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub main_screen: MainScreenState,
    pub unstaged_screen: UnstagedScreenState,
    pub running: bool,
    pub files: Vec<FileDiff>,
    pub command_history: CommandHistory,
    pub previous_commits: Vec<CommitInfo>,
    pub selected_commit_files: Vec<FileDiff>,
    pub screen: Screen,
    pub editor_request: Option<EditorRequest>,
    pub error_message: Option<String>,
}
impl AppState {
    pub fn new(repo_path: PathBuf, files: Vec<FileDiff>) -> Self {
        let commit_message =
            commit_storage::load_commit_message(&repo_path).unwrap_or_else(|_| String::new());
        let previous_commits = get_local_commits(&repo_path).unwrap_or_default();
        let selected_commit_files = previous_commits
            .first()
            .map(|c| get_commit_diff(&repo_path, &c.hash).unwrap_or_default())
            .unwrap_or_default();

        let has_unstaged_changes = git::has_unstaged_changes(&repo_path).unwrap_or(false);
        let unstaged_files = get_unstaged_diff(&repo_path);
        let untracked_files = get_untracked_files(&repo_path).unwrap_or_default();

        let mut main_screen = MainScreenState::default();
        main_screen.commit_message = commit_message;
        main_screen.has_unstaged_changes = has_unstaged_changes;
        main_screen.list_items = Self::build_main_screen_list_items(&files, &previous_commits);
        main_screen.file_cursor = if !files.is_empty() { 1 } else { 0 };

        let mut unstaged_screen = UnstagedScreenState::default();
        unstaged_screen.unstaged_files = unstaged_files.clone();
        unstaged_screen.untracked_files = untracked_files.clone();
        unstaged_screen.list_items =
            Self::build_unstaged_screen_list_items(&unstaged_files, &untracked_files);

        let mut s = Self {
            repo_path,
            main_screen,
            unstaged_screen,
            running: true,
            files,
            command_history: CommandHistory::new(),
            previous_commits,
            selected_commit_files,
            screen: Screen::Main,
            editor_request: None,
            error_message: None,
        };
        s.update_selected_commit_diff();
        s
    }

    fn build_main_screen_list_items(
        files: &[FileDiff],
        previous_commits: &[CommitInfo],
    ) -> Vec<MainScreenListItem> {
        let mut items = Vec::new();
        items.push(MainScreenListItem::StagedChangesHeader);
        for file in files {
            items.push(MainScreenListItem::File(file.clone()));
        }
        items.push(MainScreenListItem::CommitMessageInput);
        for commit in previous_commits {
            items.push(MainScreenListItem::PreviousCommitInfo {
                hash: commit.hash.clone(),
                message: commit.message.clone(),
                is_on_remote: commit.is_on_remote,
            });
        }
        items
    }

    pub fn build_unstaged_screen_list_items(
        unstaged_files: &[FileDiff],
        untracked_files: &[String],
    ) -> Vec<UnstagedScreenListItem> {
        let mut items = Vec::new();
        items.push(UnstagedScreenListItem::UnstagedChangesHeader);
        for file in unstaged_files {
            items.push(UnstagedScreenListItem::File(file.clone()));
        }
        items.push(UnstagedScreenListItem::UntrackedFilesHeader);
        for file_name in untracked_files {
            items.push(UnstagedScreenListItem::UntrackedFile(file_name.clone()));
        }
        items
    }

    pub fn get_cursor_line_index(&self) -> usize {
        if let Some(item) = self
            .main_screen
            .list_items
            .get(self.main_screen.file_cursor)
        {
            if let MainScreenListItem::File(_) = item {
                self.main_screen.line_cursor
            } else {
                0
            }
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
        self.previous_commits = get_local_commits(&self.repo_path).unwrap_or_default();

        self.main_screen.has_unstaged_changes =
            git::has_unstaged_changes(&self.repo_path).unwrap_or(false);
        let unstaged_files = get_unstaged_diff(&self.repo_path);
        let untracked_files = get_untracked_files(&self.repo_path).unwrap_or_default();

        self.main_screen.list_items =
            Self::build_main_screen_list_items(&self.files, &self.previous_commits);
        self.unstaged_screen.list_items =
            Self::build_unstaged_screen_list_items(&unstaged_files, &untracked_files);
        self.unstaged_screen.unstaged_files = unstaged_files;
        self.unstaged_screen.untracked_files = untracked_files;

        self.update_selected_commit_diff();

        if self.files.is_empty() {
            self.main_screen.file_cursor = 0;
            self.main_screen.line_cursor = 0;
            self.main_screen.diff_scroll = 0;
        } else {
            self.main_screen.file_cursor =
                old_file_cursor.min(self.main_screen.list_items.len() - 1);
            if let Some(item) = self
                .main_screen
                .list_items
                .get(self.main_screen.file_cursor)
            {
                if let MainScreenListItem::File(file) = item {
                    let max_line = file.lines.len().saturating_sub(1);
                    self.main_screen.line_cursor = old_line_cursor.min(max_line);
                    self.main_screen.diff_scroll = old_scroll.min(max_line);
                } else {
                    self.main_screen.line_cursor = 0;
                    self.main_screen.diff_scroll = 0;
                }
            } else {
                self.main_screen.line_cursor = 0;
                self.main_screen.diff_scroll = 0;
            }
        }
        self.main_screen.file_list_scroll = old_file_list_scroll;

        let max_unstaged_cursor = self.unstaged_screen.list_items.len().saturating_sub(1);
        self.unstaged_screen.unstaged_cursor = old_unstaged_cursor.min(max_unstaged_cursor);
        self.unstaged_screen.unstaged_scroll = old_unstaged_scroll;
        self.unstaged_screen.unstaged_diff_scroll = old_unstaged_diff_scroll;
    }

    pub fn execute_and_refresh(&mut self, command: Box<dyn Command>) {
        let cursor_state = CursorState::from_app_state(self);
        self.command_history.execute(command, cursor_state);
        self.refresh_diff();
    }

    pub fn update_selected_commit_diff(&mut self) {
        if let Some(item) = self.current_main_item() {
            if let MainScreenListItem::PreviousCommitInfo { hash, .. } = item {
                self.selected_commit_files =
                    get_commit_diff(&self.repo_path, hash).unwrap_or_default();
            } else {
                self.selected_commit_files.clear();
            }
        } else {
            self.selected_commit_files.clear();
        }
    }

    pub fn current_main_item(&self) -> Option<&MainScreenListItem> {
        self.main_screen
            .list_items
            .get(self.main_screen.file_cursor)
    }

    pub fn current_main_file(&self) -> Option<&FileDiff> {
        if let Some(item) = self.current_main_item() {
            if let MainScreenListItem::File(file_diff) = item {
                Some(file_diff)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_unstaged_file(&self) -> Option<&FileDiff> {
        if let Some(item) = self
            .unstaged_screen
            .list_items
            .get(self.unstaged_screen.unstaged_cursor)
        {
            if let UnstagedScreenListItem::File(file_diff) = item {
                Some(file_diff)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn main_header_height(&self, max_y: i32) -> (usize, usize) {
        let file_list_total_items = self.main_screen.list_items.len();
        let height = (max_y as usize / 3).max(3).min(file_list_total_items);
        (height, file_list_total_items)
    }

    pub fn unstaged_header_height(&self, max_y: i32) -> (usize, usize) {
        let file_list_total_items = self.unstaged_screen.list_items.len();
        let height = (max_y as usize / 3).max(3).min(file_list_total_items);
        (height, file_list_total_items)
    }
}
