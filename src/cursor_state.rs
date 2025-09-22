use crate::app_state::{AppState, Screen};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorState {
    // Screen
    pub screen: Screen,

    // Main screen
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub scroll: usize,
    pub file_list_scroll: usize,
    pub horizontal_scroll: usize,

    // Unstaged screen
    pub unstaged_cursor: usize,
    pub unstaged_scroll: usize,
    pub unstaged_diff_scroll: usize,
    pub unstaged_horizontal_scroll: usize,
}

impl CursorState {
    pub fn from_app_state(state: &AppState) -> Self {
        Self {
            screen: state.screen,
            file_cursor: state.file_cursor,
            line_cursor: state.line_cursor,
            scroll: state.diff_scroll,
            file_list_scroll: state.file_list_scroll,
            horizontal_scroll: state.horizontal_scroll,
            unstaged_cursor: state.unstaged_cursor,
            unstaged_scroll: state.unstaged_scroll,
            unstaged_diff_scroll: state.unstaged_diff_scroll,
            unstaged_horizontal_scroll: state.unstaged_horizontal_scroll,
        }
    }

    pub fn apply_to_app_state(&self, state: &mut AppState) {
        state.screen = self.screen;

        // Restore main screen cursors
        state.file_cursor = self.file_cursor.min(state.files.len() + 2);
        if let Some(file) = state.current_file() {
            state.line_cursor = self.line_cursor.min(file.lines.len().saturating_sub(1));
        } else {
            state.line_cursor = 0;
        }
        state.diff_scroll = self.scroll;
        state.file_list_scroll = self.file_list_scroll;
        state.horizontal_scroll = self.horizontal_scroll;

        // Restore unstaged screen cursors
        state.unstaged_cursor = self.unstaged_cursor.min(state.unstaged_files.len() + 1);
        // This seems to be unused in the current implementation, but we restore it anyway.
        if let Some(_file) = state.get_unstaged_file() {
            // Unstaged view doesn't have a line_cursor in the same way, it's implicit
        }
        state.unstaged_scroll = self.unstaged_scroll;
        state.unstaged_diff_scroll = self.unstaged_diff_scroll;
        state.unstaged_horizontal_scroll = self.unstaged_horizontal_scroll;
    }
}
