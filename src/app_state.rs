use crate::background::{BackgroundWorker, Response};
use crate::command::{Command, CommandHistory};
use crate::commit_storage;
use crate::cursor_state::CursorState;
use crate::git::{
    CommitInfo, FileDiff, get_commit_diff, get_diff, get_local_commits, get_unstaged_diff,
    get_untracked_files,
};
use crate::ui::main_screen::{ListItem as MainScreenListItem, UnstagedListItem};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FocusedPane {
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
    pub commit_cursor: usize,
    pub commit_scroll_offset: usize,
    pub commit_scroll_extra_space: bool,
    pub amending_commit_hash: Option<String>,
    pub is_diff_cursor_active: bool,
    pub has_unstaged_changes: bool,
    pub list_items: Vec<MainScreenListItem>,
    pub is_reordering_commits: bool,
    pub original_list_items_for_reorder: Vec<MainScreenListItem>,
}

#[derive(Default)]
pub struct UnstagedPaneState {
    pub unstaged_files: Vec<FileDiff>,
    pub untracked_files: Vec<String>,
    pub cursor: usize,
    pub scroll: usize,
    pub diff_scroll: usize,
    pub horizontal_scroll: usize,
    pub is_diff_cursor_active: bool,
    pub list_items: Vec<UnstagedListItem>,
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub main_screen: MainScreenState,
    pub unstaged_pane: UnstagedPaneState,
    pub running: bool,
    pub files: Vec<FileDiff>,
    pub command_history: CommandHistory,
    pub reorder_command_history: Option<CommandHistory>,
    pub previous_commits: Vec<CommitInfo>,
    pub selected_commit_files: Vec<FileDiff>,
    pub focused_pane: FocusedPane,
    pub editor_request: Option<EditorRequest>,
    pub error_message: Option<String>,
    pub last_interaction_time: Option<Instant>,
    pub background_worker: BackgroundWorker,
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

        let unstaged_files = get_unstaged_diff(&repo_path);
        let untracked_files = get_untracked_files(&repo_path).unwrap_or_default();
        let has_unstaged_changes = !unstaged_files.is_empty() || !untracked_files.is_empty();

        let main_screen = MainScreenState {
            commit_message,
            has_unstaged_changes,
            list_items: Self::build_main_screen_list_items(&files, &previous_commits),
            file_cursor: if !files.is_empty() { 1 } else { 0 },
            ..Default::default()
        };

        let unstaged_pane = UnstagedPaneState {
            unstaged_files: unstaged_files.clone(),
            untracked_files: untracked_files.clone(),
            list_items: Self::build_unstaged_screen_list_items(&unstaged_files, &untracked_files),
            ..Default::default()
        };

        let focused_pane = FocusedPane::Main;

        let mut s = Self {
            repo_path,
            main_screen,
            unstaged_pane,
            running: true,
            files,
            command_history: CommandHistory::new(),
            reorder_command_history: None,
            previous_commits,
            selected_commit_files,
            focused_pane,
            editor_request: None,
            error_message: None,
            last_interaction_time: None,
            background_worker: BackgroundWorker::new(),
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
                is_fixup: commit.is_fixup,
            });
        }
        items
    }

    pub fn build_unstaged_screen_list_items(
        unstaged_files: &[FileDiff],
        untracked_files: &[String],
    ) -> Vec<UnstagedListItem> {
        let mut items = Vec::new();
        items.push(UnstagedListItem::UnstagedChangesHeader);
        for file in unstaged_files {
            items.push(UnstagedListItem::File(file.clone()));
        }
        if !untracked_files.is_empty() {
            items.push(UnstagedListItem::UntrackedFilesHeader);
            for file_name in untracked_files {
                items.push(UnstagedListItem::UntrackedFile(file_name.clone()));
            }
        }
        items
    }

    pub fn get_cursor_line_index(&self) -> usize {
        if let Some(
            MainScreenListItem::File(_) | MainScreenListItem::PreviousCommitInfo { .. },
        ) = self
            .main_screen
            .list_items
            .get(self.main_screen.file_cursor)
        {
            self.main_screen.line_cursor
        } else {
            0
        }
    }

    pub fn refresh_diff(&mut self, reset_cursor: bool) {
        let old_file_cursor = self.main_screen.file_cursor;
        let old_line_cursor = self.main_screen.line_cursor;
        let old_scroll = self.main_screen.diff_scroll;
        let old_file_list_scroll = self.main_screen.file_list_scroll;
        let old_unstaged_cursor = self.unstaged_pane.cursor;
        let old_unstaged_scroll = self.unstaged_pane.scroll;
        let old_unstaged_diff_scroll = self.unstaged_pane.diff_scroll;

        self.files = get_diff(self.repo_path.clone());
        self.previous_commits = get_local_commits(&self.repo_path).unwrap_or_default();

        let unstaged_files = get_unstaged_diff(&self.repo_path);
        let untracked_files = get_untracked_files(&self.repo_path).unwrap_or_default();
        self.main_screen.has_unstaged_changes =
            !unstaged_files.is_empty() || !untracked_files.is_empty();

        self.main_screen.list_items =
            Self::build_main_screen_list_items(&self.files, &self.previous_commits);
        self.unstaged_pane.list_items =
            Self::build_unstaged_screen_list_items(&unstaged_files, &untracked_files);
        self.unstaged_pane.unstaged_files = unstaged_files;
        self.unstaged_pane.untracked_files = untracked_files;

        self.update_selected_commit_diff();

        if reset_cursor {
            self.main_screen.file_cursor = if self.main_screen.list_items.len() > 1 {
                1
            } else {
                0
            };
            self.main_screen.line_cursor = 0;
            self.main_screen.diff_scroll = 0;
        } else if self.files.is_empty() {
            self.main_screen.file_cursor = 0;
            self.main_screen.line_cursor = 0;
            self.main_screen.diff_scroll = 0;
        } else {
            self.main_screen.file_cursor =
                old_file_cursor.min(self.main_screen.list_items.len() - 1);
            if let Some(MainScreenListItem::File(file)) = self
                .main_screen
                .list_items
                .get(self.main_screen.file_cursor)
            {
                let max_line = file.lines.len().saturating_sub(1);
                self.main_screen.line_cursor = old_line_cursor.min(max_line);
                self.main_screen.diff_scroll = old_scroll.min(max_line);
            } else {
                self.main_screen.line_cursor = 0;
                self.main_screen.diff_scroll = 0;
            }
        }
        self.main_screen.file_list_scroll = old_file_list_scroll;

        let max_unstaged_cursor = self.unstaged_pane.list_items.len().saturating_sub(1);
        self.unstaged_pane.cursor = old_unstaged_cursor.min(max_unstaged_cursor);
        self.unstaged_pane.scroll = old_unstaged_scroll;
        self.unstaged_pane.diff_scroll = old_unstaged_diff_scroll;
    }

    pub fn execute_and_refresh(&mut self, command: Box<dyn Command>) {
        let cursor_state = CursorState::from_app_state(self);
        self.command_history.execute(command, cursor_state);
        self.refresh_diff(false);
    }

    pub fn execute_reorder_command(&mut self, command: Box<dyn Command>) {
        let cursor_state = CursorState::from_app_state(self);
        if let Some(history) = &mut self.reorder_command_history {
            history.execute(command, cursor_state);
        }
    }

    pub fn update_selected_commit_diff(&mut self) {
        if let Some(hash) = self.get_selected_commit_hash() {
            self.selected_commit_files =
                get_commit_diff(&self.repo_path, &hash).unwrap_or_default();
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
        if let Some(MainScreenListItem::File(file_diff)) = self.current_main_item() {
            Some(file_diff)
        } else {
            None
        }
    }

    pub fn get_unstaged_file(&self) -> Option<&FileDiff> {
        if let Some(UnstagedListItem::File(file_diff)) =
            self.unstaged_pane.list_items.get(self.unstaged_pane.cursor)
        {
            Some(file_diff)
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
        let file_list_total_items = self.unstaged_pane.list_items.len();
        let height = (max_y as usize / 3).max(3).min(file_list_total_items);
        (height, file_list_total_items)
    }

    pub fn is_in_input_mode(&self) -> bool {
        matches!(
            self.current_main_item(),
            Some(MainScreenListItem::CommitMessageInput)
                | Some(MainScreenListItem::AmendingCommitMessageInput { .. })
                | Some(MainScreenListItem::EditingReorderCommit { .. })
        )
    }

    pub fn debounce_diff_update(&mut self) {
        self.last_interaction_time = Some(Instant::now());
    }

    pub fn check_diff_update(&mut self) -> bool {
        if let Some(last_time) = self.last_interaction_time {
            if last_time.elapsed() > std::time::Duration::from_millis(200) {
                if let Some(hash) = self.get_selected_commit_hash() {
                    self.background_worker
                        .request_commit_diff(self.repo_path.clone(), hash);
                }
                self.last_interaction_time = None;
                return false; // Don't trigger render yet, wait for response
            }
        }
        false
    }

    pub fn poll_background(&mut self) -> bool {
        let mut needs_render = false;
        while let Some(response) = self.background_worker.poll() {
            match response {
                Response::CommitDiff(hash, diff) => {
                    if let Some(current_hash) = self.get_selected_commit_hash() {
                        if current_hash == hash {
                            self.selected_commit_files = diff;
                            needs_render = true;
                        }
                    }
                }
            }
        }
        needs_render
    }

    pub fn get_selected_commit_hash(&self) -> Option<String> {
        if let Some(item) = self.current_main_item() {
            match item {
                MainScreenListItem::PreviousCommitInfo { hash, .. } => Some(hash.clone()),
                MainScreenListItem::EditingReorderCommit { hash, .. } => Some(hash.clone()),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn jump_to_file_in_diff(&mut self) -> bool {
        if !self.main_screen.is_diff_cursor_active {
            return false;
        }

        if self.selected_commit_files.is_empty() {
            return false;
        }

        let line_index = self.main_screen.line_cursor;

        // The first file contains the header which includes the stat summary.
        let first_file = &self.selected_commit_files[0];
        if line_index >= first_file.lines.len() {
            // We are already beyond the first file's lines (including header),
            // so we are definitely not in the stat summary.
            return false;
        }

        let mut stat_line_indices = Vec::new();
        let mut patch_start_index = 0;

        for (i, line) in first_file.lines.iter().enumerate() {
            if line.starts_with("diff --git ") {
                patch_start_index = i;
                break;
            }
            // Heuristic for stat line: " path | count +++---"
            if line.contains('|') {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 2 {
                    let right = parts[1].trim();
                    if right.contains('+')
                        || right.contains('-')
                        || right.contains("Bin")
                        || (!right.is_empty()
                            && right.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()))
                    {
                        stat_line_indices.push(i);
                    }
                }
            }
        }

        if let Some(stat_index) = stat_line_indices.iter().position(|&idx| idx == line_index) {
            // Found the file index (stat_index)
            let mut target_offset = 0;
            if stat_index == 0 {
                target_offset = patch_start_index;
            } else if stat_index < self.selected_commit_files.len() {
                // Sum lengths of previous files
                for i in 0..stat_index {
                    target_offset += self.selected_commit_files[i].lines.len();
                }
            } else {
                return false;
            }

            self.main_screen.line_cursor = target_offset;

            // Update diff_scroll to show the jumped-to line at the top
            self.main_screen.diff_scroll = target_offset;

            return true;
        }

        false
    }
}
